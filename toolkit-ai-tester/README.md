# AI自动化测试员 开发规范

## 简介

以Rust的RIG框架为基础, 搭建一个由AI驱动, 操作已有的Toolkit Studio以及其tke内核进行App观察和操作的, 完全无须人类主动干预就能进行App UI测试任务的AI自动化测试员  

## 架构

基于Rust语言的LLM框架RIG搭建, 文档见 https://docs.rs/rig-core/latest/rig/  

### AI Agent分工

### Receptionist

### Librarian

### Worker

### Supervisor


### 处理流程

1. 用户在Project页面, 进入一个project, 在测试用例列表中选择一项测试用例, 选择让AI测试  
2. 将该测试用例信息传递给AI Agent Receptionist 和 Librarian  
3. Receptionist负责对该测试用例进行理解, Librarian负责去已经存储的本地知识里查找可能与此测试用例相关的知识并提取出来  
4. 开始测试, 但是先不要让Worker开始, 而是先获取截图和xml到workarea和正常执行测试一样(截图获取后需要展示在前端上)  
5. 使用tke将xml优化提取, 使用OCR服务将截图中的文字提取出来  
6. 将Receptionist的分析和测试要求 以及 Librarian找到的用户存储的本地知识一起作为Worker的prompt, 构建本次测试执行的专属worker
7. 将提取的xml和截图ocr后获取的文字内容, 发给worker, 作为第一次观察  
8. Worker进行决策, 也就是 对什么元素(xml或ocr文字) 进行什么操作(参考tke能够执行的操作The_ToolkitScript_Reference.md)  
9. 通过tke执行本次操作  
10. 执行后, 将本次操作append在本次AI测试的记录脚本 .tks文件中  
11. 然后再获取截图和xml到workarea, 作为下一次观察(此时需要刷新前端的截图展示)  
12. 继续提取xml, 获取截图ocr, worker进行下一轮tke执行, 记录到.tks; 一轮一轮持续进行  
13. 当worker觉得测试已经结束时, 将自己的判断和最后几轮的对话session发送给Supervisor进行判断  
14. 如果Supervisor判定这个测试已经完成, 就真的结束测试, AI测试员程序退出; 但是如果Supervisor觉得没完成, 就告诉worker继续测试, 并且要求应该怎么测试, worker则接着去继续测试, 直到再通报supervisor并且通过审核, 才能结束  


## 开发规范

必须要单元化, 去耦合, 一个代码文件仅允许包含一个方向的代码, 如果超出500行就必须拆分成多个文件, 并且放置在有逻辑和有组织的文件夹结构中  

如果有问题就直接抛出报错, 禁止使用default值, 模拟数据, 临时假数据等  
