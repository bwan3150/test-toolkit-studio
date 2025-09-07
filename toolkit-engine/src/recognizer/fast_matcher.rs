// 快速图像匹配模块 - 优化的模板匹配实现

use crate::{Result, TkeError, Point};
use image::{DynamicImage, GenericImageView, Luma, Pixel};
use rayon::prelude::*;
use std::path::PathBuf;
use tracing::{debug, info};

pub struct FastMatcher {
    confidence_threshold: f32,
    coarse_step: u32,     // 粗搜索步长
    fine_search_range: i32, // 精搜索范围
}

impl FastMatcher {
    pub fn new() -> Self {
        Self {
            confidence_threshold: 0.75,
            coarse_step: 10,        // 粗搜索时每10个像素采样
            fine_search_range: 15,   // 精搜索范围15像素
        }
    }
    
    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.confidence_threshold = threshold;
    }
    
    // 执行快速模板匹配
    pub fn match_template(&self, screenshot_path: &PathBuf, template_path: &PathBuf) -> Result<Point> {
        // 加载图像
        let screenshot = image::open(screenshot_path)
            .map_err(|e| TkeError::ImageError(format!("加载截图失败: {}", e)))?;
        let template = image::open(template_path)
            .map_err(|e| TkeError::ImageError(format!("加载模板失败: {}", e)))?;
        
        let screen_width = screenshot.width();
        let screen_height = screenshot.height();
        let template_width = template.width();
        let template_height = template.height();
        
        info!("截图尺寸: {}x{}, 模板尺寸: {}x{}", 
              screen_width, screen_height, template_width, template_height);
        
        if template_width > screen_width || template_height > screen_height {
            return Err(TkeError::InvalidArgument("模板图片大于截图".to_string()));
        }
        
        // 转换为灰度图以提高速度
        let screenshot_gray = screenshot.to_luma8();
        let template_gray = template.to_luma8();
        
        // 第一步：粗搜索找到大致区域
        let coarse_match = self.coarse_search(&screenshot_gray, &template_gray)?;
        debug!("粗搜索找到位置: {:?}, 相似度: {:.3}", coarse_match.position, coarse_match.similarity);
        
        // 第二步：在最佳匹配点附近精确搜索
        let fine_match = self.fine_search(
            &screenshot_gray, 
            &template_gray, 
            coarse_match.position
        )?;
        
        info!("精搜索找到最佳位置: {:?}, 相似度: {:.3}", 
              fine_match.position, fine_match.similarity);
        
        // 检查置信度
        if fine_match.similarity < self.confidence_threshold {
            return Err(TkeError::ElementNotFound(
                format!("图像匹配置信度不足: {:.3} < {:.3}", 
                       fine_match.similarity, self.confidence_threshold)
            ));
        }
        
        // 计算中心坐标
        let center_x = fine_match.position.x + template_width as i32 / 2;
        let center_y = fine_match.position.y + template_height as i32 / 2;
        
        Ok(Point::new(center_x, center_y))
    }
    
    // 粗搜索 - 使用较大步长快速扫描
    fn coarse_search(
        &self, 
        screenshot: &image::GrayImage, 
        template: &image::GrayImage
    ) -> Result<MatchResult> {
        let screen_width = screenshot.width();
        let screen_height = screenshot.height();
        let template_width = template.width();
        let template_height = template.height();
        
        let search_width = screen_width - template_width + 1;
        let search_height = screen_height - template_height + 1;
        
        // 并行搜索
        let step = self.coarse_step as i32;
        let positions: Vec<(i32, i32)> = (0..search_height as i32)
            .step_by(step as usize)
            .flat_map(|y| {
                (0..search_width as i32)
                    .step_by(step as usize)
                    .map(move |x| (x, y))
            })
            .collect();
        
        let results: Vec<MatchResult> = positions
            .par_iter()
            .map(|&(x, y)| {
                let similarity = self.calculate_similarity_fast(
                    screenshot, 
                    template, 
                    x as u32, 
                    y as u32
                );
                MatchResult {
                    position: Point::new(x, y),
                    similarity,
                }
            })
            .collect();
        
        // 找到最佳匹配
        results.into_iter()
            .max_by(|a, b| a.similarity.partial_cmp(&b.similarity).unwrap())
            .ok_or_else(|| TkeError::ElementNotFound("粗搜索未找到匹配".to_string()))
    }
    
    // 精搜索 - 在粗搜索结果附近精确匹配
    fn fine_search(
        &self,
        screenshot: &image::GrayImage,
        template: &image::GrayImage,
        coarse_position: Point
    ) -> Result<MatchResult> {
        let screen_width = screenshot.width() as i32;
        let screen_height = screenshot.height() as i32;
        let template_width = template.width() as i32;
        let template_height = template.height() as i32;
        
        // 确定精搜索范围
        let start_x = (coarse_position.x - self.fine_search_range).max(0);
        let end_x = (coarse_position.x + self.fine_search_range)
            .min(screen_width - template_width);
        let start_y = (coarse_position.y - self.fine_search_range).max(0);
        let end_y = (coarse_position.y + self.fine_search_range)
            .min(screen_height - template_height);
        
        // 生成所有搜索位置
        let positions: Vec<(i32, i32)> = (start_y..=end_y)
            .flat_map(|y| (start_x..=end_x).map(move |x| (x, y)))
            .collect();
        
        // 并行计算相似度
        let results: Vec<MatchResult> = positions
            .par_iter()
            .map(|&(x, y)| {
                let similarity = self.calculate_similarity_accurate(
                    screenshot,
                    template,
                    x as u32,
                    y as u32
                );
                MatchResult {
                    position: Point::new(x, y),
                    similarity,
                }
            })
            .collect();
        
        // 找到最佳匹配
        results.into_iter()
            .max_by(|a, b| a.similarity.partial_cmp(&b.similarity).unwrap())
            .ok_or_else(|| TkeError::ElementNotFound("精搜索未找到匹配".to_string()))
    }
    
    // 快速相似度计算 - 采样计算
    fn calculate_similarity_fast(
        &self,
        screenshot: &image::GrayImage,
        template: &image::GrayImage,
        x: u32,
        y: u32
    ) -> f32 {
        let template_width = template.width();
        let template_height = template.height();
        
        let mut sum_diff: f64 = 0.0;
        let mut count = 0;
        
        // 每5个像素采样一次
        let sample_step = 5;
        
        for ty in (0..template_height).step_by(sample_step) {
            for tx in (0..template_width).step_by(sample_step) {
                let screen_pixel = screenshot.get_pixel(x + tx, y + ty);
                let template_pixel = template.get_pixel(tx, ty);
                
                let diff = (screen_pixel[0] as i32 - template_pixel[0] as i32).abs() as f64;
                sum_diff += diff;
                count += 1;
            }
        }
        
        if count == 0 {
            return 0.0;
        }
        
        // 归一化相似度
        let avg_diff = sum_diff / count as f64;
        let similarity = 1.0 - (avg_diff / 255.0);
        
        similarity as f32
    }
    
    // 精确相似度计算 - 全像素计算但使用优化算法
    fn calculate_similarity_accurate(
        &self,
        screenshot: &image::GrayImage,
        template: &image::GrayImage,
        x: u32,
        y: u32
    ) -> f32 {
        let template_width = template.width();
        let template_height = template.height();
        
        // 使用归一化互相关（NCC）
        let mut sum_st: f64 = 0.0;
        let mut sum_s2: f64 = 0.0;
        let mut sum_t2: f64 = 0.0;
        
        // 计算模板均值
        let template_mean = self.calculate_mean(template);
        
        // 计算截图区域均值
        let mut region_sum: f64 = 0.0;
        let mut pixel_count = 0;
        
        for ty in 0..template_height {
            for tx in 0..template_width {
                let pixel = screenshot.get_pixel(x + tx, y + ty)[0] as f64;
                region_sum += pixel;
                pixel_count += 1;
            }
        }
        
        let region_mean = region_sum / pixel_count as f64;
        
        // 计算归一化互相关
        for ty in 0..template_height {
            for tx in 0..template_width {
                let s = screenshot.get_pixel(x + tx, y + ty)[0] as f64 - region_mean;
                let t = template.get_pixel(tx, ty)[0] as f64 - template_mean;
                
                sum_st += s * t;
                sum_s2 += s * s;
                sum_t2 += t * t;
            }
        }
        
        // 避免除零
        if sum_s2 == 0.0 || sum_t2 == 0.0 {
            return 0.0;
        }
        
        // 归一化互相关系数
        let ncc = sum_st / (sum_s2.sqrt() * sum_t2.sqrt());
        
        // 转换到0-1范围
        ((ncc + 1.0) / 2.0) as f32
    }
    
    // 计算图像均值
    fn calculate_mean(&self, image: &image::GrayImage) -> f64 {
        let mut sum: f64 = 0.0;
        let mut count = 0;
        
        for pixel in image.pixels() {
            sum += pixel[0] as f64;
            count += 1;
        }
        
        sum / count as f64
    }
}

struct MatchResult {
    position: Point,
    similarity: f32,
}