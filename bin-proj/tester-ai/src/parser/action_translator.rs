// Action Translator - 将 AI 决策转换为 TKE 命令和 .tks 脚本

use crate::models::{ActionDecision, ActionType, AssertCondition, Direction, ScreenState, ElementType};
use crate::parser::worker_parser::WorkerParser;
use crate::parser::element_manager::ElementManager;
use anyhow::{Context, Result};

/// 操作转换器
pub struct ActionTranslator;

impl ActionTranslator {
    /// 将 AI 决策转换为 TKE 命令参数
    ///
    /// 返回: (命令名, 参数列表)
    pub fn translate_to_tke_command(
        decision: &ActionDecision,
        parser: &WorkerParser,
    ) -> Result<(String, Vec<String>)> {
        match decision.action_type {
            ActionType::Click => {
                let element_id = decision.target_element_id
                    .context("Click 操作需要目标元素 ID")?;
                let pos = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;

                Ok(("tap".to_string(), vec![pos.0.to_string(), pos.1.to_string()]))
            }

            ActionType::Press => {
                let element_id = decision.target_element_id
                    .context("Press 操作需要目标元素 ID")?;
                let pos = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;
                let duration = decision.params.duration.unwrap_or(1000);

                // TKE 的 press 命令参数需要确认
                // 假设使用 tap + 长按实现
                Ok(("tap".to_string(), vec![pos.0.to_string(), pos.1.to_string()]))
            }

            ActionType::Swipe => {
                let element_id = decision.target_element_id
                    .context("Swipe 操作需要起点元素 ID")?;
                let from = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;
                let to = decision.params.to
                    .context("Swipe 操作需要终点坐标")?;
                let duration = decision.params.duration.unwrap_or(1000);

                Ok(("swipe".to_string(), vec![
                    from.0.to_string(),
                    from.1.to_string(),
                    to.0.to_string(),
                    to.1.to_string(),
                    "--duration".to_string(),
                    duration.to_string(),
                ]))
            }

            ActionType::Drag => {
                let element_id = decision.target_element_id
                    .context("Drag 操作需要起点元素 ID")?;
                let from = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;
                let to = decision.params.to
                    .context("Drag 操作需要终点坐标")?;
                let duration = decision.params.duration.unwrap_or(1000);

                Ok(("swipe".to_string(), vec![
                    from.0.to_string(),
                    from.1.to_string(),
                    to.0.to_string(),
                    to.1.to_string(),
                    "--duration".to_string(),
                    duration.to_string(),
                ]))
            }

            ActionType::DirectionalDrag => {
                let element_id = decision.target_element_id
                    .context("DirectionalDrag 操作需要起点元素 ID")?;
                let from = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;
                let direction = decision.params.direction.as_ref()
                    .context("DirectionalDrag 操作需要方向")?;
                let distance = decision.params.distance
                    .context("DirectionalDrag 操作需要距离")?;
                let duration = decision.params.duration.unwrap_or(1000);

                // 计算终点坐标
                let to = match direction {
                    Direction::Up => (from.0, from.1 - distance),
                    Direction::Down => (from.0, from.1 + distance),
                    Direction::Left => (from.0 - distance, from.1),
                    Direction::Right => (from.0 + distance, from.1),
                };

                Ok(("swipe".to_string(), vec![
                    from.0.to_string(),
                    from.1.to_string(),
                    to.0.to_string(),
                    to.1.to_string(),
                    "--duration".to_string(),
                    duration.to_string(),
                ]))
            }

            ActionType::Input => {
                let text = decision.params.text.as_ref()
                    .context("Input 操作需要文本")?;

                Ok(("input".to_string(), vec![text.clone()]))
            }

            ActionType::Clear => {
                // TKE 没有直接的 clear 命令, 需要通过点击 + 清空实现
                let element_id = decision.target_element_id
                    .context("Clear 操作需要目标元素 ID")?;
                let pos = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;

                Ok(("tap".to_string(), vec![pos.0.to_string(), pos.1.to_string()]))
            }

            ActionType::HideKeyboard => {
                // TKE 可能有隐藏键盘的命令
                Ok(("back".to_string(), vec![]))
            }

            ActionType::Wait => {
                let duration = decision.params.duration.unwrap_or(1000);
                // TKE 没有 wait 命令, 需要在脚本层面实现
                Ok(("wait".to_string(), vec![duration.to_string()]))
            }

            ActionType::Back => {
                Ok(("back".to_string(), vec![]))
            }

            ActionType::Launch => {
                let package = decision.params.package.as_ref()
                    .context("Launch 操作需要包名")?;
                let activity = decision.params.activity.as_ref()
                    .context("Launch 操作需要 Activity")?;

                Ok(("launch".to_string(), vec![package.clone(), activity.clone()]))
            }

            ActionType::Stop => {
                let package = decision.params.package.as_ref()
                    .context("Stop 操作需要包名")?;

                Ok(("stop".to_string(), vec![package.clone()]))
            }

            ActionType::Assert | ActionType::ReadText => {
                // Assert 和 ReadText 暂时不转换为 TKE 命令
                // 直接在脚本层面记录
                Ok(("assert".to_string(), vec![]))
            }

            ActionType::None => {
                // 测试完成，无需操作
                Ok(("wait".to_string(), vec!["0".to_string()]))
            }
        }
    }

    /// 将 AI 决策转换为 .tks 脚本行（使用元素名）
    pub fn translate_to_tks_script_with_element_names(
        decision: &ActionDecision,
        parser: &WorkerParser,
        element_manager: &mut ElementManager,
        screen_state: &ScreenState,
    ) -> Result<String> {
        match decision.action_type {
            ActionType::Click => {
                let element_id = decision.target_element_id
                    .context("Click 操作需要目标元素 ID")?;

                // 添加元素到管理器并获取元素名
                let element_name = Self::add_element_to_manager(
                    element_id,
                    parser,
                    element_manager,
                    screen_state,
                )?;

                Ok(format!("点击 [{{{}}}]", element_name))
            }

            ActionType::Press => {
                let element_id = decision.target_element_id
                    .context("Press 操作需要目标元素 ID")?;
                let duration = decision.params.duration.unwrap_or(1000);

                let element_name = Self::add_element_to_manager(
                    element_id,
                    parser,
                    element_manager,
                    screen_state,
                )?;

                Ok(format!("按压 [{{{}}}, {}]", element_name, duration))
            }

            ActionType::Swipe | ActionType::Drag => {
                let element_id = decision.target_element_id
                    .context("Swipe/Drag 操作需要起点元素 ID")?;
                let from = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;
                let to = decision.params.to
                    .context("Swipe/Drag 操作需要终点坐标")?;
                let duration = decision.params.duration.unwrap_or(1000);

                if matches!(decision.action_type, ActionType::Swipe) {
                    Ok(format!("滑动 [{{{},{}}}, {{{},{}}}, {}]",
                        from.0, from.1, to.0, to.1, duration))
                } else {
                    Ok(format!("拖动 [{{{},{}}}, {{{},{}}}, {}]",
                        from.0, from.1, to.0, to.1, duration))
                }
            }

            ActionType::DirectionalDrag => {
                let element_id = decision.target_element_id
                    .context("DirectionalDrag 操作需要起点元素 ID")?;
                let from = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;
                let direction = decision.params.direction.as_ref()
                    .context("DirectionalDrag 操作需要方向")?;
                let distance = decision.params.distance
                    .context("DirectionalDrag 操作需要距离")?;
                let duration = decision.params.duration.unwrap_or(1000);

                let dir_str = match direction {
                    Direction::Up => "up",
                    Direction::Down => "down",
                    Direction::Left => "left",
                    Direction::Right => "right",
                };

                Ok(format!("定向拖动 [{{{},{}}}, {}, {}, {}]",
                    from.0, from.1, dir_str, distance, duration))
            }

            ActionType::Input => {
                let element_id = decision.target_element_id
                    .context("Input 操作需要目标元素 ID")?;
                let text = decision.params.text.as_ref()
                    .context("Input 操作需要文本")?;

                let element_name = Self::add_element_to_manager(
                    element_id,
                    parser,
                    element_manager,
                    screen_state,
                )?;

                Ok(format!("输入 [{{{}}}, {}]", element_name, text))
            }

            ActionType::Clear => {
                let element_id = decision.target_element_id
                    .context("Clear 操作需要目标元素 ID")?;

                let element_name = Self::add_element_to_manager(
                    element_id,
                    parser,
                    element_manager,
                    screen_state,
                )?;

                Ok(format!("清理 [{{{}}}]", element_name))
            }

            ActionType::HideKeyboard => {
                Ok("隐藏键盘".to_string())
            }

            ActionType::Wait => {
                let duration = decision.params.duration.unwrap_or(1000);
                Ok(format!("等待 [{}]", duration))
            }

            ActionType::Back => {
                Ok("返回".to_string())
            }

            ActionType::Launch => {
                let package = decision.params.package.as_ref()
                    .context("Launch 操作需要包名")?;
                let activity = decision.params.activity.as_ref()
                    .context("Launch 操作需要 Activity")?;

                Ok(format!("启动 [{}, {}]", package, activity))
            }

            ActionType::Stop => {
                let package = decision.params.package.as_ref()
                    .context("Stop 操作需要包名")?;
                let empty_activity = String::new();
                let activity = decision.params.activity.as_ref().unwrap_or(&empty_activity);

                Ok(format!("关闭 [{}, {}]", package, activity))
            }

            ActionType::Assert => {
                let element_id = decision.target_element_id
                    .context("Assert 操作需要目标元素 ID")?;
                let condition = decision.params.assert_condition.as_ref()
                    .context("Assert 操作需要断言条件")?;

                let condition_str = match condition {
                    AssertCondition::Exists => "存在",
                    AssertCondition::NotExists => "不存在",
                    AssertCondition::Visible => "可见",
                    AssertCondition::NotVisible => "不可见",
                };

                Ok(format!("断言 [{{元素{}}}, {}]", element_id, condition_str))
            }

            ActionType::ReadText => {
                let element_id = decision.target_element_id
                    .context("ReadText 操作需要目标元素 ID")?;
                let pos = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;

                Ok(format!("读取 [{{{},{}}}]", pos.0, pos.1))
            }

            ActionType::None => {
                // 测试完成，无需操作
                Ok("# 测试完成".to_string())
            }
        }
    }

    /// 添加元素到管理器并返回元素名
    fn add_element_to_manager(
        element_id: usize,
        parser: &WorkerParser,
        element_manager: &mut ElementManager,
        screen_state: &ScreenState,
    ) -> Result<String> {
        let element_info = parser.get_element_info(element_id)
            .context(format!("找不到元素 ID {}", element_id))?;

        // 根据元素类型找到原始元素信息
        let merged_element = screen_state.merged_elements
            .iter()
            .find(|e| e.id == element_id)
            .context(format!("找不到元素 ID {}", element_id))?;

        let element_name = match merged_element.element_type {
            ElementType::Xml => {
                // 从 UI 元素列表中找到对应的元素
                let ui_element = &screen_state.ui_elements[merged_element.original_index];

                element_manager.add_element_from_xml(
                    element_id as u64,
                    &ui_element.class_name,
                    ui_element.text.as_deref(),
                    ui_element.resource_id.as_deref(),
                    (
                        ui_element.bounds.x1,
                        ui_element.bounds.y1,
                        ui_element.bounds.x2,
                        ui_element.bounds.y2,
                    ),
                    ui_element.clickable,
                )
            }
            ElementType::Ocr => {
                // 从 OCR 文本列表中找到对应的文本
                let ocr_text = &screen_state.ocr_texts[merged_element.original_index];

                // 从 bbox 计算边界
                let (min_x, min_y, max_x, max_y) = ocr_text.bbox.iter().fold(
                    (i32::MAX, i32::MAX, i32::MIN, i32::MIN),
                    |(min_x, min_y, max_x, max_y), point| {
                        (
                            min_x.min(point[0] as i32),
                            min_y.min(point[1] as i32),
                            max_x.max(point[0] as i32),
                            max_y.max(point[1] as i32),
                        )
                    },
                );

                element_manager.add_element_from_ocr(
                    element_id as u64,
                    &ocr_text.text,
                    (min_x, min_y, max_x, max_y),
                )
            }
        };

        Ok(element_name)
    }

    /// 将 AI 决策转换为 .tks 脚本行（旧版本，使用坐标）
    #[allow(dead_code)]
    pub fn translate_to_tks_script(
        decision: &ActionDecision,
        parser: &WorkerParser,
    ) -> Result<String> {
        match decision.action_type {
            ActionType::Click => {
                let element_id = decision.target_element_id
                    .context("Click 操作需要目标元素 ID")?;
                let pos = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;

                Ok(format!("点击 [{{{}{}}}]", pos.0, pos.1))
            }

            ActionType::Press => {
                let element_id = decision.target_element_id
                    .context("Press 操作需要目标元素 ID")?;
                let pos = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;
                let duration = decision.params.duration.unwrap_or(1000);

                Ok(format!("按压 [{{{},{}}}, {}]", pos.0, pos.1, duration))
            }

            ActionType::Swipe | ActionType::Drag => {
                let element_id = decision.target_element_id
                    .context("Swipe/Drag 操作需要起点元素 ID")?;
                let from = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;
                let to = decision.params.to
                    .context("Swipe/Drag 操作需要终点坐标")?;
                let duration = decision.params.duration.unwrap_or(1000);

                if matches!(decision.action_type, ActionType::Swipe) {
                    Ok(format!("滑动 [{{{},{}}}, {{{},{}}}, {}]",
                        from.0, from.1, to.0, to.1, duration))
                } else {
                    Ok(format!("拖动 [{{{},{}}}, {{{},{}}}, {}]",
                        from.0, from.1, to.0, to.1, duration))
                }
            }

            ActionType::DirectionalDrag => {
                let element_id = decision.target_element_id
                    .context("DirectionalDrag 操作需要起点元素 ID")?;
                let from = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;
                let direction = decision.params.direction.as_ref()
                    .context("DirectionalDrag 操作需要方向")?;
                let distance = decision.params.distance
                    .context("DirectionalDrag 操作需要距离")?;
                let duration = decision.params.duration.unwrap_or(1000);

                let dir_str = match direction {
                    Direction::Up => "up",
                    Direction::Down => "down",
                    Direction::Left => "left",
                    Direction::Right => "right",
                };

                Ok(format!("定向拖动 [{{{},{}}}, {}, {}, {}]",
                    from.0, from.1, dir_str, distance, duration))
            }

            ActionType::Input => {
                let element_id = decision.target_element_id
                    .context("Input 操作需要目标元素 ID")?;
                let pos = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;
                let text = decision.params.text.as_ref()
                    .context("Input 操作需要文本")?;

                Ok(format!("输入 [{{{},{}}}, {}]", pos.0, pos.1, text))
            }

            ActionType::Clear => {
                let element_id = decision.target_element_id
                    .context("Clear 操作需要目标元素 ID")?;
                let pos = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;

                Ok(format!("清理 [{{{}{}}}]", pos.0, pos.1))
            }

            ActionType::HideKeyboard => {
                Ok("隐藏键盘".to_string())
            }

            ActionType::Wait => {
                let duration = decision.params.duration.unwrap_or(1000);
                Ok(format!("等待 [{}]", duration))
            }

            ActionType::Back => {
                Ok("返回".to_string())
            }

            ActionType::Launch => {
                let package = decision.params.package.as_ref()
                    .context("Launch 操作需要包名")?;
                let activity = decision.params.activity.as_ref()
                    .context("Launch 操作需要 Activity")?;

                Ok(format!("启动 [{}, {}]", package, activity))
            }

            ActionType::Stop => {
                let package = decision.params.package.as_ref()
                    .context("Stop 操作需要包名")?;
                let empty_activity = String::new();
                let activity = decision.params.activity.as_ref().unwrap_or(&empty_activity);

                Ok(format!("关闭 [{}, {}]", package, activity))
            }

            ActionType::Assert => {
                let element_id = decision.target_element_id
                    .context("Assert 操作需要目标元素 ID")?;
                let condition = decision.params.assert_condition.as_ref()
                    .context("Assert 操作需要断言条件")?;

                let condition_str = match condition {
                    AssertCondition::Exists => "存在",
                    AssertCondition::NotExists => "不存在",
                    AssertCondition::Visible => "可见",
                    AssertCondition::NotVisible => "不可见",
                };

                // 这里简化处理, 实际需要根据元素类型生成不同的脚本
                Ok(format!("断言 [{{元素{}}}, {}]", element_id, condition_str))
            }

            ActionType::ReadText => {
                let element_id = decision.target_element_id
                    .context("ReadText 操作需要目标元素 ID")?;
                let pos = parser.get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;

                Ok(format!("读取 [{{{},{}}}]", pos.0, pos.1))
            }

            ActionType::None => {
                // 测试完成，无需操作
                Ok("# 测试完成".to_string())
            }
        }
    }
}
