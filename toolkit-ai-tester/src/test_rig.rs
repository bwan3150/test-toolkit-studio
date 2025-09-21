// 测试 rig-core 导入

#[cfg(test)]
mod tests {
    #[test]
    fn test_rig_import() {
        // 简单测试是否能导入 rig-core
        println!("测试 rig-core 导入");
    }
}

// 尝试导入 rig-core
use rig_core;

pub fn test_function() {
    println!("rig-core 导入成功");
}