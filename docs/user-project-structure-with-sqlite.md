  项目目录/
  ├── cases/              # 测试用例
  ├── workarea/           # 临时文件（截图、UI树XML）
  ├── img/                # 图像识别截图
  ├── project.db          # SQLite 数据库
  └── testcase_sheet.csv  # 测试用例表

  数据库表

  - locators - 元素定位信息
  - testcases - 测试用例映射
  - devices - 项目级设备配置

  创建的文件

  | 文件                               | 说明                 |
  |----------------------------------|--------------------|
  | handlers/database/db-core.js     | SQLite 核心连接管理      |
  | handlers/database/locator-db.js  | locator 表 CRUD 操作  |
  | handlers/database/testcase-db.js | testcase 表 CRUD 操作 |
  | handlers/database/device-db.js   | device 表 CRUD 操作   |
  | handlers/database/index.js       | 统一入口和注册            |

  修改的文件

  | 文件                                                          | 变更               |
  |-------------------------------------------------------------|------------------|
  | main.js                                                     | 注册数据库处理器，退出时关闭连接 |
  | handlers/project/project-handlers.js                        | 创建新项目结构 + SQLite |
  | renderer/js/testcase/panels/locator-library-panel.js        | 改用数据库 IPC        |
  | renderer/js/modals/element-edit-modal.js                    | 改用数据库 IPC        |
  | renderer/js/testcase/screen/modes/screenshot-mode.js        | 新路径 img/         |
  | renderer/js/testcase/codejar/block-mode/block-ui-builder.js | 从 locator 获取路径   |
  | renderer/js/testcase/codejar/block-mode-editor.js           | 从 locator 获取路径   |

  删除的文件

  - renderer/js/testcase/controller/locator-manager-tke.js （未使用的遗留代码）

  IPC 接口

  // Locator
  'db-locator-getAll'    // 获取所有
  'db-locator-get'       // 获取单个
  'db-locator-save'      // 保存
  'db-locator-delete'    // 删除
  'db-locator-batchSave' // 批量保存

  // Testcase
  'db-testcase-getAll', 'db-testcase-getMap', 'db-testcase-create', ...

  // Device
  'db-device-getAll', 'db-device-save', 'db-device-delete', ...

