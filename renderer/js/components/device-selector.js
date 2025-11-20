// 设备下拉列表组件
// 统一管理所有页面的设备选择器，自动加载设备并使用保存的友好名称
//
// 用法示例：
//   const selector = new DeviceSelector('deviceSelect');
//   await selector.refresh();
//   selector.onChange((deviceId) => { console.log('选中设备:', deviceId); });

(function() {
    'use strict';

    class DeviceSelector {
        /**
         * 创建设备选择器实例
         * @param {string|HTMLElement} selectElement - select元素或其ID
         * @param {Object} options - 配置选项
         * @param {string} options.emptyText - 无设备时的提示文本
         * @param {boolean} options.autoRestore - 是否自动恢复上次选择
         * @param {string} options.storageKey - 存储选择的key
         */
        constructor(selectElement, options = {}) {
            // 获取select元素
            if (typeof selectElement === 'string') {
                this.selectElement = document.getElementById(selectElement);
            } else {
                this.selectElement = selectElement;
            }

            if (!this.selectElement) {
                throw new Error('设备选择器: 找不到select元素');
            }

            // 配置选项
            this.options = {
                emptyText: 'Select Device',
                autoRestore: true,
                storageKey: 'selected_device',
                ...options
            };

            // 变更监听器列表
            this.changeListeners = [];

            // 绑定change事件
            this.selectElement.addEventListener('change', () => {
                this._handleChange();
            });

            window.rLog('✅ DeviceSelector已初始化:', this.selectElement.id);
        }

        /**
         * 刷新设备列表
         * @returns {Promise<void>}
         */
        async refresh() {
            const { ipcRenderer, path, fs, yaml } = window.AppGlobals;

            try {
                // 使用get-connected-devices API获取已连接设备
                const result = await ipcRenderer.invoke('get-connected-devices');

                if (!result || !result.success) {
                    window.rError('获取设备列表失败:', result?.error);
                    this._renderEmpty();
                    return;
                }

                const connectedDevices = result.devices || [];

                // 获取保存的设备配置（用于显示友好名称）
                let deviceConfigs = {};
                const currentProject = window.AppGlobals.currentProject;

                if (currentProject) {
                    try {
                        const devicesPath = path.join(currentProject, 'devices');

                        // 检查devices文件夹是否存在
                        let devicesPathExists = false;
                        try {
                            await fs.access(devicesPath);
                            devicesPathExists = true;
                        } catch {
                            // devices文件夹不存在，跳过
                            window.rLog('devices文件夹不存在，跳过加载设备配置');
                        }

                        if (devicesPathExists) {
                            const files = await fs.readdir(devicesPath);
                            for (const file of files) {
                                if (file.endsWith('.yaml')) {
                                    const filePath = path.join(devicesPath, file);
                                    const content = await fs.readFile(filePath, 'utf-8');
                                    const config = yaml.load(content);
                                    if (config.deviceId) {
                                        deviceConfigs[config.deviceId] = config.deviceName || config.deviceId;
                                    }
                                }
                            }
                        }
                    } catch (error) {
                        window.rWarn('加载设备配置失败:', error.message);
                        deviceConfigs = {};
                    }
                }

                // 渲染设备列表
                this._renderDevices(connectedDevices, deviceConfigs);

                // 自动恢复上次选择
                if (this.options.autoRestore) {
                    await this._restoreSelection();
                }

                window.rLog('✅ 设备列表已刷新，共', connectedDevices.length, '个设备');

            } catch (error) {
                window.rError('刷新设备列表失败:', error);
                this._renderEmpty();
            }
        }

        /**
         * 渲染设备列表
         * @private
         */
        _renderDevices(devices, deviceConfigs) {
            // 清空现有选项
            this.selectElement.innerHTML = `<option value="">${this.options.emptyText}</option>`;

            if (!devices || devices.length === 0) {
                return;
            }

            // 添加设备选项
            devices.forEach(device => {
                const option = document.createElement('option');
                option.value = device.id;

                // 优先使用保存的设备名称，否则使用model或id
                let displayName = device.id;
                if (deviceConfigs[device.id]) {
                    displayName = deviceConfigs[device.id];
                } else if (device.model) {
                    displayName = device.model;
                }

                option.textContent = displayName;
                this.selectElement.appendChild(option);
            });
        }

        /**
         * 渲染空列表
         * @private
         */
        _renderEmpty() {
            this.selectElement.innerHTML = `<option value="">${this.options.emptyText}</option>`;
        }

        /**
         * 恢复上次选择的设备
         * @private
         */
        async _restoreSelection() {
            const { ipcRenderer } = window.AppGlobals;

            try {
                const savedSelection = await ipcRenderer.invoke('store-get', this.options.storageKey);

                if (savedSelection) {
                    // 检查设备是否还在列表中
                    const options = Array.from(this.selectElement.options);
                    const optionExists = options.some(opt => opt.value === savedSelection);

                    if (optionExists) {
                        this.selectElement.value = savedSelection;
                        window.rLog('✅ 已恢复设备选择:', savedSelection);

                        // 触发change事件
                        this._handleChange();
                    } else {
                        window.rLog('上次选择的设备不在列表中:', savedSelection);
                    }
                }
            } catch (error) {
                window.rError('恢复设备选择失败:', error);
            }
        }

        /**
         * 处理设备变更
         * @private
         */
        async _handleChange() {
            const selectedValue = this.selectElement.value;

            // 保存选择
            if (selectedValue) {
                const { ipcRenderer } = window.AppGlobals;
                try {
                    await ipcRenderer.invoke('store-set', this.options.storageKey, selectedValue);
                } catch (error) {
                    window.rError('保存设备选择失败:', error);
                }
            }

            // 触发所有监听器
            this.changeListeners.forEach(listener => {
                try {
                    listener(selectedValue);
                } catch (error) {
                    window.rError('设备变更监听器执行失败:', error);
                }
            });
        }

        /**
         * 注册设备变更监听器
         * @param {Function} callback - 回调函数，参数为选中的deviceId
         */
        onChange(callback) {
            if (typeof callback === 'function') {
                this.changeListeners.push(callback);
            }
        }

        /**
         * 移除设备变更监听器
         * @param {Function} callback - 要移除的回调函数
         */
        offChange(callback) {
            const index = this.changeListeners.indexOf(callback);
            if (index > -1) {
                this.changeListeners.splice(index, 1);
            }
        }

        /**
         * 获取当前选中的设备ID
         * @returns {string} 设备ID，如果未选择则返回空字符串
         */
        getSelectedDevice() {
            return this.selectElement.value;
        }

        /**
         * 设置选中的设备
         * @param {string} deviceId - 要选中的设备ID
         */
        setSelectedDevice(deviceId) {
            this.selectElement.value = deviceId;
            this._handleChange();
        }

        /**
         * 获取设备数量
         * @returns {number} 设备数量（不包括"Select Device"选项）
         */
        getDeviceCount() {
            return this.selectElement.options.length - 1;
        }

        /**
         * 销毁选择器
         */
        destroy() {
            this.changeListeners = [];
            // select元素由DOM管理，不需要手动移除
        }
    }

    // 导出到全局
    window.DeviceSelector = DeviceSelector;

    window.rLog('✅ DeviceSelector组件已加载');

})();
