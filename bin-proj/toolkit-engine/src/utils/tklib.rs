// 【.tklib 脚本元素包】每个 .tks 脚本自持一个打包好的元素库——`foo.tks ↔ foo.tklib`。
//
// 设计（方案定稿 2026-07-03）：**没有共享元素库**。一个测试 = 两个文件，复制到别的机器即可跑：
//   foo.tks     人读的脚本（纯文本，轻量）
//   foo.tklib   元素包（zip 容器、stored 不压缩——png 本身已压缩）：
//     ├── meta.json      录制平台/设备/tke 版本/创建时间（先只记录不消费，给跨分辨率适配留钩子）
//     ├── element.json   元素条目（结构/OCR 通道 + img 相对引用）
//     └── img/*.png      图像模板（三级降级的兜底通道）
//
// 运行期是「解包 → 操作 → 回包」生命周期（像 docx）：装配层把 tklib 解包到 cache 临时目录、
// 把 element_path 指过去——recognizer/element 工具/ScriptRunner **零改动**；有修改（repair 落
// 新元素等）再重新打包写回。zip 内布局与解包后的库目录布局完全一致（element.json + img/）。

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Result, TkeError};

/// tklib 的元信息（打包时记录；当前只写不读，供人检查与后续版本/分辨率适配用）
#[derive(Debug, Serialize, Deserialize)]
pub struct TklibMeta {
    /// 录制平台（android / ios / web）
    pub platform: String,
    /// 录制设备 id
    pub device: String,
    /// 打包时的 tke 版本
    pub tke_version: String,
    /// 创建时间（RFC3339）
    pub created: String,
}

impl TklibMeta {
    pub fn new(platform: &str, device: &str) -> Self {
        Self {
            platform: platform.to_string(),
            device: device.to_string(),
            tke_version: env!("CARGO_PKG_VERSION").to_string(),
            created: chrono::Local::now().to_rfc3339(),
        }
    }
}

/// `foo.tks` → 同目录 `foo.tklib`
pub fn tklib_path(tks: &Path) -> PathBuf {
    tks.with_extension("tklib")
}

fn zip_err(what: &str, e: impl std::fmt::Display) -> TkeError {
    TkeError::InvalidArgument(format!("{}: {}", what, e))
}

/// 打包：把 `lib_json`（element.json）与其同目录的 `img/` 模板打成 `out_tklib`。
/// 只收录 element.json 里 img 字段实际引用到的图（不整目录扫，避免带上孤儿图片）。
pub fn pack(lib_json: &Path, out_tklib: &Path, meta: &TklibMeta) -> Result<()> {
    let lib_dir = lib_json.parent().unwrap_or_else(|| Path::new("."));
    let lib_content = std::fs::read_to_string(lib_json).map_err(TkeError::IoError)?;
    let lib: serde_json::Value = serde_json::from_str(&lib_content)
        .map_err(|e| TkeError::InvalidArgument(format!("元素库解析失败 {}: {}", lib_json.display(), e)))?;

    if let Some(parent) = out_tklib.parent() {
        std::fs::create_dir_all(parent).map_err(TkeError::IoError)?;
    }
    let file = std::fs::File::create(out_tklib).map_err(TkeError::IoError)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // meta.json
    zip.start_file("meta.json", opts).map_err(|e| zip_err("tklib 写入 meta 失败", e))?;
    zip.write_all(serde_json::to_string_pretty(meta).unwrap_or_default().as_bytes())
        .map_err(TkeError::IoError)?;

    // element.json（原样收录）
    zip.start_file("element.json", opts).map_err(|e| zip_err("tklib 写入 element.json 失败", e))?;
    zip.write_all(lib_content.as_bytes()).map_err(TkeError::IoError)?;

    // pages/*：页面实体的截图（pages 节 img 字段引用，如 "pages/起始页.png"）
    if let Some(pages) = lib["pages"].as_object() {
        for (name, entry) in pages {
            let Some(img_rel) = entry["img"].as_str() else { continue };
            let src = lib_dir.join(img_rel);
            if let Ok(bytes) = std::fs::read(&src) {
                zip.start_file(img_rel, opts).map_err(|e| zip_err("tklib 写入页面截图失败", e))?;
                zip.write_all(&bytes).map_err(TkeError::IoError)?;
            } else {
                tracing::warn!("页面 {} 的截图缺失: {}", name, src.display());
            }
        }
    }

    // img/*：只收录条目实际引用的模板图（img 字段是相对库目录的路径，如 "img/xxx.png"）
    if let Some(elements) = lib["elements"].as_object() {
        for (name, entry) in elements {
            let Some(img_rel) = entry["img"].as_str() else { continue };
            let src = lib_dir.join(img_rel);
            match std::fs::read(&src) {
                Ok(bytes) => {
                    zip.start_file(img_rel, opts).map_err(|e| zip_err("tklib 写入模板图失败", e))?;
                    zip.write_all(&bytes).map_err(TkeError::IoError)?;
                }
                Err(e) => {
                    // 图缺失不炸整个打包（该元素退化为结构/OCR 定位），但绝不静默
                    eprintln!("⚠ tklib 打包：元素「{}」的模板图缺失（{}: {}），已跳过", name, src.display(), e);
                }
            }
        }
    }

    zip.finish().map_err(|e| zip_err("tklib 收尾失败", e))?;
    Ok(())
}

/// 解包到 `dest_dir`（自动建目录），返回解包后的 element.json 路径——把它当 element_path
/// 传给下游（recognizer/工具/回放），下游对 tklib 无感知。
pub fn unpack(tklib: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let file = std::fs::File::open(tklib)
        .map_err(|e| TkeError::InvalidArgument(format!("打不开元素包 {}: {}", tklib.display(), e)))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| zip_err("tklib 不是有效的 zip", e))?;
    std::fs::create_dir_all(dest_dir).map_err(TkeError::IoError)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| zip_err("tklib 读取条目失败", e))?;
        // 防 zip-slip：enclosed_name 拒绝绝对路径与 `..`
        let Some(rel) = entry.enclosed_name() else {
            return Err(TkeError::InvalidArgument(format!("tklib 含非法路径条目：{}", entry.name())));
        };
        let out = dest_dir.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(TkeError::IoError)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(TkeError::IoError)?;
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes).map_err(TkeError::IoError)?;
        std::fs::write(&out, bytes).map_err(TkeError::IoError)?;
    }

    let lib_json = dest_dir.join("element.json");
    if !lib_json.is_file() {
        return Err(TkeError::InvalidArgument(format!("元素包 {} 里没有 element.json", tklib.display())));
    }
    Ok(lib_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 打包→解包 往返：element.json 原样、引用的 img 带上、meta 在场
    #[test]
    fn pack_unpack_roundtrip() {
        let base = std::env::temp_dir().join(format!("tke-tklib-test-{}", std::process::id()));
        let src = base.join("src");
        let _ = std::fs::create_dir_all(src.join("img"));
        let lib = r#"{"elements":{"按钮@1_2":{"img":"img/按钮@1_2.png","desc":"测试"}}}"#;
        std::fs::write(src.join("element.json"), lib).unwrap();
        std::fs::write(src.join("img/按钮@1_2.png"), b"fake-png-bytes").unwrap();

        let pkg = base.join("case.tklib");
        pack(&src.join("element.json"), &pkg, &TklibMeta::new("android", "fake:x")).unwrap();
        assert!(pkg.is_file());

        let dest = base.join("dest");
        let lib_json = unpack(&pkg, &dest).unwrap();
        assert_eq!(std::fs::read_to_string(&lib_json).unwrap(), lib);
        assert_eq!(std::fs::read(dest.join("img/按钮@1_2.png")).unwrap(), b"fake-png-bytes");
        let meta: TklibMeta = serde_json::from_str(&std::fs::read_to_string(dest.join("meta.json")).unwrap()).unwrap();
        assert_eq!(meta.platform, "android");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// tks → tklib 路径映射
    #[test]
    fn tklib_path_maps_extension() {
        assert_eq!(tklib_path(Path::new("/a/b/foo.tks")), PathBuf::from("/a/b/foo.tklib"));
    }
}
