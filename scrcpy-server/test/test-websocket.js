// 测试 WebSocket 连接
const WebSocket = require('ws');

const url = 'ws://localhost:8000/?action=proxy-adb&remote=tcp%3A8886&udid=f64b3b4d';
console.log('连接到:', url);

const ws = new WebSocket(url);

ws.on('open', () => {
  console.log('✅ WebSocket 连接成功!');
});

ws.on('error', (error) => {
  console.error('❌ WebSocket 错误:', error.message);
  console.error('详细信息:', error);
});

ws.on('close', (code, reason) => {
  console.log('WebSocket 关闭:', code, reason.toString());
});

ws.on('message', (data) => {
  console.log('收到消息:', data.length, 'bytes');
});

setTimeout(() => {
  console.log('超时，关闭连接');
  ws.close();
  process.exit(0);
}, 10000);
