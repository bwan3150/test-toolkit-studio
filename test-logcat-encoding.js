// Windows平台logcat编码测试工具
// 用于测试和诊断logcat输出的编码问题

const { spawn } = require('child_process');
const iconv = require('iconv-lite');
const path = require('path');

// 获取ADB路径（根据项目结构）
function getAdbPath() {
  const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
  const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
  return path.join(__dirname, 'resources', platform, 'android-sdk', 'platform-tools', adbName);
}

// 测试logcat编码
async function testLogcatEncoding() {
  console.log('========================================');
  console.log('Windows Logcat编码测试工具');
  console.log('平台:', process.platform);
  console.log('========================================\n');

  const adbPath = getAdbPath();
  console.log('ADB路径:', adbPath);

  // 首先获取连接的设备
  const { execSync } = require('child_process');
  let devices;
  
  try {
    const devicesOutput = execSync(`"${adbPath}" devices`).toString();
    const lines = devicesOutput.split('\n').filter(line => line.includes('\t'));
    devices = lines.map(line => line.split('\t')[0]).filter(id => id);
    
    if (devices.length === 0) {
      console.log('错误: 没有连接的Android设备');
      console.log('请确保设备已连接并开启USB调试');
      return;
    }
    
    console.log('找到设备:', devices.join(', '));
    console.log('使用设备:', devices[0]);
  } catch (error) {
    console.error('获取设备列表失败:', error.message);
    return;
  }

  const device = devices[0];
  
  // 测试直接调用adb（不使用cmd包装）
  console.log('\n测试方案: 直接调用ADB（不使用CMD包装）');
  console.log('----------------------------------------');
  
  const command = adbPath;
  const args = ['-s', device, 'logcat', '-v', 'threadtime'];
  
  const spawnOptions = {
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: false  // 不使用shell
  };
  
  console.log('执行命令:', command, args.join(' '));
  const logcatProcess = spawn(command, args, spawnOptions);
  
  let lineCount = 0;
  let hasChineseContent = false;
  let hasEncodingIssues = false;
  const maxLines = 50; // 只测试前50行
  
  logcatProcess.stdout.on('data', (data) => {
    if (lineCount >= maxLines) {
      logcatProcess.kill();
      return;
    }
    
    let output;
    
    // 测试GBK解码
    try {
      const gbkOutput = iconv.decode(data, 'gbk');
      const utf8Output = data.toString('utf8');
      
      // 检查GBK解码质量
      const gbkHasError = gbkOutput.includes('\ufffd') || /[\uFFFD\u0000-\u001F]/g.test(gbkOutput);
      const utf8HasError = utf8Output.includes('\ufffd');
      
      if (!gbkHasError) {
        output = gbkOutput;
        console.log(`[行 ${++lineCount}] 使用GBK编码成功`);
      } else if (!utf8HasError) {
        output = utf8Output;
        console.log(`[行 ${++lineCount}] 使用UTF-8编码成功`);
      } else {
        // 两种都有问题，选择错误较少的
        output = gbkOutput;
        hasEncodingIssues = true;
        console.log(`[行 ${++lineCount}] 编码有问题，使用GBK（fallback）`);
      }
      
      // 检查是否包含中文
      if (/[\u4e00-\u9fa5]/.test(output)) {
        hasChineseContent = true;
        const lines = output.split('\n');
        const sampleLine = lines[0].substring(0, 100);
        console.log('  示例（含中文）:', sampleLine);
      }
      
      // 如果有乱码，显示样本
      if (output.includes('\ufffd') || /[σ╖▓σÉ»σè¿]/u.test(output)) {
        const lines = output.split('\n');
        const sampleLine = lines[0].substring(0, 100);
        console.log('  ⚠️ 检测到乱码:', sampleLine);
        hasEncodingIssues = true;
      }
      
    } catch (error) {
      console.error('编码处理错误:', error.message);
    }
  });
  
  logcatProcess.stderr.on('data', (data) => {
    console.error('Logcat错误:', data.toString());
  });
  
  logcatProcess.on('close', (code) => {
    console.log('\n========================================');
    console.log('测试结果总结:');
    console.log('----------------------------------------');
    console.log('测试行数:', lineCount);
    console.log('包含中文内容:', hasChineseContent ? '是' : '否');
    console.log('存在编码问题:', hasEncodingIssues ? '是' : '否');
    
    if (!hasEncodingIssues) {
      console.log('\n✅ 编码问题已解决！');
      console.log('建议: 使用当前的GBK优先解码方案');
    } else {
      console.log('\n⚠️ 仍存在编码问题');
      console.log('建议: 可能需要进一步调试或尝试其他编码');
    }
    
    console.log('\n测试完成！');
  });
  
  // 10秒后自动停止测试
  setTimeout(() => {
    if (logcatProcess.exitCode === null) {
      console.log('\n测试时间到，停止logcat...');
      logcatProcess.kill();
    }
  }, 10000);
}

// 运行测试
if (require.main === module) {
  testLogcatEncoding().catch(console.error);
}

module.exports = { testLogcatEncoding };
