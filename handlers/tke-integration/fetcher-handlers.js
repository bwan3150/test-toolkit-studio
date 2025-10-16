// TKE Fetcher 模块的 IPC 处理器
// 负责 UI 元素提取、屏幕尺寸推断、UI 树生成等
// 注意：此模块只负责执行 TKE 命令并返回原始输出，JSON 解析在 renderer 层完成
const { ipcMain } = require('electron');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const { getTkePath } = require('./adb-handlers');

// 通用的 TKE 命令执行函数（支持 stdin 输入）
async function execTkeCommandWithStdin(tkePath, args, stdinData = null) {
  return new Promise((resolve, reject) => {
    const child = spawn(tkePath, args, {
      stdio: ['pipe', 'pipe', 'pipe']
    });

    let stdout = '';
    let stderr = '';

    child.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    child.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    child.on('close', (exitCode) => {
      if (exitCode !== 0) {
        reject(new Error(`TKE命令失败 (exit code ${exitCode}): ${stderr}`));
        return;
      }

      if (stderr && !stderr.includes('INFO')) {
        console.warn('TKE命令警告:', stderr);
      }

      // 验证输出是否为 JSON
      const output = stdout.trim();
      try {
        JSON.parse(output);
        resolve(output); // 返回原始 JSON 字符串
      } catch (e) {
        reject(new Error(`TKE 输出不是有效的 JSON: ${output}`));
      }
    });

    // 如果有 stdin 数据，写入
    if (stdinData) {
      child.stdin.write(stdinData);
    }
    child.stdin.end();
  });
}

// 注册 TKE Fetcher 相关的 IPC 处理器
function registerFetcherHandlers(app) {
  // 提取 UI 元素 - tke fetcher extract-ui-elements --width <w> --height <h> < xml_file
  ipcMain.handle('tke-fetcher-extract-ui-elements', async (event, options) => {
    try {
      const { projectPath, screenWidth, screenHeight } = options;

      if (!projectPath || !path.isAbsolute(projectPath)) {
        return { success: false, error: '请提供有效的项目路径（绝对路径）' };
      }

      if (!screenWidth || !screenHeight) {
        return { success: false, error: '请提供屏幕宽度和高度' };
      }

      const tkePath = getTkePath(app);
      if (!fs.existsSync(tkePath)) {
        return { success: false, error: 'TKE可执行文件未找到' };
      }

      // XML文件路径
      const xmlPath = path.join(projectPath, 'workarea', 'current_ui_tree.xml');
      if (!fs.existsSync(xmlPath)) {
        return { success: false, error: 'UI XML文件不存在，请先截图' };
      }

      // 构建命令参数
      const args = [
        'fetcher', 'extract-ui-elements',
        '--width', screenWidth.toString(),
        '--height', screenHeight.toString()
      ];

      // 读取 XML 文件内容
      const xmlContent = fs.readFileSync(xmlPath, 'utf8');

      console.log('执行TKE fetcher extract-ui-elements');
      const output = await execTkeCommandWithStdin(tkePath, args, xmlContent);

      return {
        success: true,
        output: output // 返回原始 JSON 字符串
      };

    } catch (error) {
      console.error('TKE fetcher extract-ui-elements失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 推断屏幕尺寸 - tke fetcher infer-screen-size < xml_file
  ipcMain.handle('tke-fetcher-infer-screen-size', async (event, projectPath) => {
    try {
      if (!projectPath || !path.isAbsolute(projectPath)) {
        return { success: false, error: '请提供有效的项目路径（绝对路径）' };
      }

      const tkePath = getTkePath(app);
      if (!fs.existsSync(tkePath)) {
        return { success: false, error: 'TKE可执行文件未找到' };
      }

      // XML文件路径
      const xmlPath = path.join(projectPath, 'workarea', 'current_ui_tree.xml');
      if (!fs.existsSync(xmlPath)) {
        return { success: false, error: 'UI XML文件不存在，请先截图' };
      }

      // 构建命令参数
      const args = ['fetcher', 'infer-screen-size'];

      // 读取 XML 文件内容
      const xmlContent = fs.readFileSync(xmlPath, 'utf8');

      console.log('执行TKE fetcher infer-screen-size');
      const output = await execTkeCommandWithStdin(tkePath, args, xmlContent);

      return {
        success: true,
        output: output
      };

    } catch (error) {
      console.error('TKE fetcher infer-screen-size失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 生成 UI 树字符串 - tke fetcher generate-tree-string < xml_file
  // 注意：此命令返回纯文本，不是 JSON
  ipcMain.handle('tke-fetcher-generate-tree-string', async (event, projectPath) => {
    try {
      if (!projectPath || !path.isAbsolute(projectPath)) {
        return { success: false, error: '请提供有效的项目路径（绝对路径）' };
      }

      const tkePath = getTkePath(app);
      if (!fs.existsSync(tkePath)) {
        return { success: false, error: 'TKE可执行文件未找到' };
      }

      // XML文件路径
      const xmlPath = path.join(projectPath, 'workarea', 'current_ui_tree.xml');
      if (!fs.existsSync(xmlPath)) {
        return { success: false, error: 'UI XML文件不存在，请先截图' };
      }

      // 构建命令参数
      const args = ['fetcher', 'generate-tree-string'];

      // 读取 XML 文件内容
      const xmlContent = fs.readFileSync(xmlPath, 'utf8');

      return new Promise((resolve, reject) => {
        const child = spawn(tkePath, args, {
          stdio: ['pipe', 'pipe', 'pipe']
        });

        let stdout = '';
        let stderr = '';

        child.stdout.on('data', (data) => {
          stdout += data.toString();
        });

        child.stderr.on('data', (data) => {
          stderr += data.toString();
        });

        child.on('close', (exitCode) => {
          if (exitCode !== 0) {
            console.error('TKE generate-tree-string 命令失败:', stderr);
            resolve({ success: false, error: `TKE命令失败: ${stderr}` });
            return;
          }

          if (stderr && !stderr.includes('INFO')) {
            console.warn('TKE命令警告:', stderr);
          }

          console.log('TKE生成UI树字符串成功');
          resolve({
            success: true,
            output: stdout // 返回纯文本输出
          });
        });

        // 写入 XML 内容到 stdin
        child.stdin.write(xmlContent);
        child.stdin.end();
      });

    } catch (error) {
      console.error('TKE fetcher generate-tree-string失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerFetcherHandlers
};
