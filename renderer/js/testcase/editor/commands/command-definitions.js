// 命令定义 - 所有可用命令的元数据
// 包含命令类型、图标、颜色和参数定义

const BlockDefinitions = {
    application: {
        color: '#c586c0',
        icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M16.56,5.44L15.11,6.89C16.84,7.94 18,9.83 18,12A6,6 0 0,1 12,18A6,6 0 0,1 6,12C6,9.83 7.16,7.94 8.88,6.88L7.44,5.44C5.36,6.88 4,9.28 4,12A8,8 0 0,0 12,20A8,8 0 0,0 20,12C20,9.28 18.64,6.88 16.56,5.44M13,3H11V13H13"/></svg>',
        commands: [
            {
                type: '启动',
                label: '启动',
                tksCommand: '启动',
                params: [
                    { name: '包名', type: 'text', placeholder: '包名', default: 'com.example.test_toolkit' },
                    { name: 'Activity', type: 'text', placeholder: 'Activity名字', default: '.MainActivity' }
                ]
            },
            {
                type: '关闭',
                label: '关闭',
                tksCommand: '关闭',
                params: [
                    { name: '包名', type: 'text', placeholder: '包名', default: 'com.example.app' },
                    { name: 'Activity', type: 'text', placeholder: 'Activity名字', default: '.MainActivity' }
                ]
            }
        ]
    },
    action: {
        color: '#569cd6',
        icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 14a8 8 0 0 1-8 8"/><path d="M18 11v-1a2 2 0 0 0-2-2a2 2 0 0 0-2 2"/><path d="M14 10V9a2 2 0 0 0-2-2a2 2 0 0 0-2 2v1"/><path d="M10 9.5V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v10"/><path d="M18 11a2 2 0 1 1 4 0v3a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15"/></svg>',
        commands: [
            {
                type: '点击',
                label: '点击',
                tksCommand: '点击',
                params: [
                    { name: '元素', type: 'element', placeholder: '坐标/XML/图片元素', default: '' }
                ]
            },
            {
                type: '按压',
                label: '按压',
                tksCommand: '按压',
                params: [
                    { name: '元素', type: 'element', placeholder: '坐标/XML/图片元素', default: '' },
                    { name: '时长/ms', type: 'number', placeholder: '持续时长/ms', default: '1000' }
                ]
            },
            {
                type: '滑动',
                label: '滑动',
                tksCommand: '滑动',
                params: [
                    { name: '起点', type: 'coordinate', placeholder: '起点坐标', default: '{200,400}' },
                    { name: '终点', type: 'coordinate', placeholder: '终点坐标', default: '{300,600}' },
                    { name: '时长/ms', type: 'number', placeholder: '持续时长/ms', default: '1000' }
                ]
            },
            {
                type: '拖动',
                label: '拖动',
                tksCommand: '拖动',
                params: [
                    { name: '元素', type: 'element', placeholder: '坐标/XML/图片元素', default: '' },
                    { name: '终点', type: 'coordinate', placeholder: '终点坐标', default: '{500,800}' },
                    { name: '时长/ms', type: 'number', placeholder: '持续时长/ms', default: '1000' }
                ]
            },
            {
                type: '定向拖动',
                label: '定向拖动',
                tksCommand: '定向拖动',
                params: [
                    { name: '元素', type: 'element', placeholder: '坐标/XML/图片元素', default: '' },
                    { name: '方向', type: 'select', placeholder: '方向', default: 'up', options: ['up', 'down', 'left', 'right'] },
                    { name: '距离', type: 'number', placeholder: '拖动距离', default: '300' },
                    { name: '时长/ms', type: 'number', placeholder: '持续时长/ms', default: '1000' }
                ]
            }
        ]
    },
    input: {
        color: '#4ec9b0',
        icon: '⌨',
        commands: [
            {
                type: '输入',
                label: '输入',
                tksCommand: '输入',
                params: [
                    { name: '输入框', type: 'element', placeholder: '坐标/XML元素', default: '' },
                    { name: '文本', type: 'text', placeholder: '输入的文本内容', default: '' }
                ]
            },
            {
                type: '清理',
                label: '清理',
                tksCommand: '清理',
                params: [
                    { name: '输入框', type: 'element', placeholder: '坐标/XML元素', default: '' }
                ]
            },
            {
                type: '隐藏键盘',
                label: '隐藏键盘',
                tksCommand: '隐藏键盘',
                params: []
            }
        ]
    },
    control: {
        color: '#ce9178',
        icon: '⏱',
        commands: [
            {
                type: '等待',
                label: '等待',
                tksCommand: '等待',
                params: [
                    { name: '时长/ms', type: 'number', placeholder: '等待时长/ms', default: '1000' }
                ]
            },
            {
                type: '返回',
                label: '返回',
                tksCommand: '返回',
                params: []
            }
        ]
    },
    assertion: {
        color: '#f48771',
        icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z"/></svg>',
        commands: [
            {
                type: '断言',
                label: '断言',
                tksCommand: '断言',
                params: [
                    { name: '元素', type: 'element', placeholder: 'XML/图片元素', default: '' },
                    { name: '状态', type: 'select', options: ['存在', '不存在', '可见', '不可见'], default: '存在' }
                ]
            }
        ]
    },
    text: {
        color: '#9cdcfe',
        icon: '📖',
        commands: [
            {
                type: '读取',
                label: '读取',
                tksCommand: '读取',
                params: [
                    { name: '中心坐标或元素', type: 'element', placeholder: '坐标/XML元素', default: '' },
                    { name: '宽度', type: 'number', placeholder: '左右扩展', default: '' },
                    { name: '高度', type: 'number', placeholder: '上下扩展', default: '' }
                ]
            }
        ]
    }
};

// 导出到全局
window.BlockDefinitions = BlockDefinitions;

if (window.rLog) {
    window.rLog('✅ BlockDefinitions 模块已加载');
}
