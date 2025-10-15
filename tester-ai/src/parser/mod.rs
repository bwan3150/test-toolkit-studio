// Worker Parser 模块 - 将屏幕信息转换为 AI 可理解的格式

mod worker_parser;
mod action_translator;
mod element_manager;

pub use worker_parser::WorkerParser;
pub use action_translator::ActionTranslator;
pub use element_manager::{ElementManager, ElementDefinition};
