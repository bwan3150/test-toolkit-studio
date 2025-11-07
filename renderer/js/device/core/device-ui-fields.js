// 设备管理UI字段显示控制
// 管理设备表单中不同字段的显示/隐藏逻辑

/**
 * 更新连接类型字段显示（简化版本，主要逻辑已移至 updatePlatformFields）
 * 依赖：updatePlatformFields
 */
function updateConnectionTypeFields() {
    // 触发平台字段更新，因为连接类型会影响字段显示
    updatePlatformFields();
}

/**
 * 更新平台字段显示 - 使用组合键方式
 * 根据选择的平台和连接类型动态显示相应的配置字段
 */
function updatePlatformFields() {
    const platformRadio = document.querySelector('input[name="platform"]:checked');
    const connectionRadio = document.querySelector('input[name="connectionType"]:checked');

    if (!platformRadio || !connectionRadio) return;

    const platform = platformRadio.value;
    const connectionType = connectionRadio.value;

    // 生成组合键
    const configKey = `${platform}-${connectionType}`;

    // 隐藏所有配置组并禁用required属性
    document.querySelectorAll('.config-group').forEach(group => {
        group.style.display = 'none';
        // 禁用隐藏组中的required字段
        const requiredInputs = group.querySelectorAll('input[required]');
        requiredInputs.forEach(input => {
            input.disabled = true;
        });
    });

    // 显示对应的配置组并启用required属性
    const targetConfig = document.getElementById(`${configKey}-config`);
    if (targetConfig) {
        targetConfig.style.display = 'block';
        // 启用显示组中的required字段
        const requiredInputs = targetConfig.querySelectorAll('input[required]');
        requiredInputs.forEach(input => {
            input.disabled = false;
        });
        window.rLog(`显示配置组: ${configKey}-config`);
    } else {
        console.warn(`未找到配置组: ${configKey}-config`);
    }

    window.rLog(`更新字段显示: ${configKey}`);
}

/**
 * 切换高级设置显示
 * 展开或收起高级设置区域
 */
function toggleAdvancedSettings() {
    const content = document.getElementById('advancedSettingsContent');
    const toggle = document.querySelector('.advanced-toggle');
    const icon = toggle.querySelector('.toggle-icon');

    if (content.style.display === 'none') {
        content.style.display = 'block';
        icon.style.transform = 'rotate(180deg)';
    } else {
        content.style.display = 'none';
        icon.style.transform = 'rotate(0deg)';
    }
}

// 全局函数导出
window.toggleAdvancedSettings = toggleAdvancedSettings;

// 导出函数（用于模块化引用）
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        updateConnectionTypeFields,
        updatePlatformFields,
        toggleAdvancedSettings
    };
}
