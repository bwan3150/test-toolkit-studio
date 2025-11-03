// 视频设置类 - 从 ws-scrcpy VideoSettings.ts 转换
// 用于配置视频流参数

class VideoSettings {
  constructor(params = {}) {
    this.bitrate = params.bitrate || 524288;          // 比特率 (默认 512kbps)
    this.maxFps = params.maxFps || 24;                // 最大帧率
    this.iFrameInterval = params.iFrameInterval || 5; // I帧间隔 (秒)
    this.bounds = params.bounds || null;              // 分辨率限制 {width, height}
    this.crop = params.crop || null;                  // 裁剪 {left, top, right, bottom}
    this.sendFrameMeta = params.sendFrameMeta !== undefined ? params.sendFrameMeta : false;
    this.lockedVideoOrientation = params.lockedVideoOrientation !== undefined ? params.lockedVideoOrientation : -1;
    this.displayId = params.displayId || 0;
    this.codecOptions = params.codecOptions || '';
    this.encoderName = params.encoderName || '';
  }

  /**
   * 转换为二进制 Buffer (Big Endian)
   * 格式: 35 字节固定 + 可变长度的 codecOptions 和 encoderName
   */
  toBuffer() {
    // 计算总长度
    const codecOptionsBytes = this.codecOptions ? this.stringToUtf8(this.codecOptions) : null;
    const encoderNameBytes = this.encoderName ? this.stringToUtf8(this.encoderName) : null;
    const codecOptionsLen = codecOptionsBytes ? codecOptionsBytes.length : 0;
    const encoderNameLen = encoderNameBytes ? encoderNameBytes.length : 0;

    const totalLength = 35 + codecOptionsLen + encoderNameLen;
    const buffer = new ArrayBuffer(totalLength);
    const view = new DataView(buffer);
    const u8arr = new Uint8Array(buffer);

    let offset = 0;

    // bitrate: 4 字节 (BE)
    view.setInt32(offset, this.bitrate, false);
    offset += 4;

    // maxFps: 4 字节 (BE)
    view.setInt32(offset, this.maxFps, false);
    offset += 4;

    // iFrameInterval: 1 字节
    view.setInt8(offset, this.iFrameInterval);
    offset += 1;

    // bounds: width(2) + height(2) (BE)
    const boundsWidth = this.bounds?.width || 0;
    const boundsHeight = this.bounds?.height || 0;
    view.setInt16(offset, boundsWidth, false);
    offset += 2;
    view.setInt16(offset, boundsHeight, false);
    offset += 2;

    // crop: left(2) + top(2) + right(2) + bottom(2) (BE)
    const cropLeft = this.crop?.left || 0;
    const cropTop = this.crop?.top || 0;
    const cropRight = this.crop?.right || 0;
    const cropBottom = this.crop?.bottom || 0;
    view.setInt16(offset, cropLeft, false);
    offset += 2;
    view.setInt16(offset, cropTop, false);
    offset += 2;
    view.setInt16(offset, cropRight, false);
    offset += 2;
    view.setInt16(offset, cropBottom, false);
    offset += 2;

    // sendFrameMeta: 1 字节
    view.setInt8(offset, this.sendFrameMeta ? 1 : 0);
    offset += 1;

    // lockedVideoOrientation: 1 字节
    view.setInt8(offset, this.lockedVideoOrientation);
    offset += 1;

    // displayId: 4 字节 (BE)
    view.setInt32(offset, this.displayId, false);
    offset += 4;

    // codecOptions length + data: 4 字节长度 (BE) + 数据
    view.setInt32(offset, codecOptionsLen, false);
    offset += 4;
    if (codecOptionsBytes) {
      u8arr.set(codecOptionsBytes, offset);
      offset += codecOptionsLen;
    }

    // encoderName length + data: 4 字节长度 (BE) + 数据
    view.setInt32(offset, encoderNameLen, false);
    offset += 4;
    if (encoderNameBytes) {
      u8arr.set(encoderNameBytes, offset);
      offset += encoderNameLen;
    }

    return new Uint8Array(buffer);
  }

  // UTF-8 编码辅助函数
  stringToUtf8(str) {
    const encoder = new TextEncoder();
    return encoder.encode(str);
  }
}

// 导出到全局
window.ScrcpyVideoSettings = VideoSettings;
