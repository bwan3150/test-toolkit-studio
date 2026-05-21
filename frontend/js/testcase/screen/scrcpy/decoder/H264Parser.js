// H.264 NAL 单元解析器 - 从 ws-scrcpy WebCodecsPlayer.ts 提取
// 用于解析 H.264 视频流的 NAL 单元

class H264Parser {
  // NAL 单元类型
  static NALU_TYPE_SLICE = 1;       // P-frame
  static NALU_TYPE_IDR = 5;         // I-frame (关键帧)
  static NALU_TYPE_SEI = 6;         // 补充增强信息
  static NALU_TYPE_SPS = 7;         // 序列参数集
  static NALU_TYPE_PPS = 8;         // 图像参数集

  /**
   * 获取 NAL 单元类型
   * @param {Uint8Array} data - NAL 单元数据
   * @returns {number} NAL 类型
   */
  static getNaluType(data) {
    if (data.length < 5) {
      return -1;
    }
    // NAL header 在第5个字节, 低5位是类型
    return data[4] & 0x1F;
  }

  /**
   * 解析 SPS (Sequence Parameter Set)
   * 提取视频的宽度、高度和 codec 字符串
   * @param {Uint8Array} spsData - SPS NAL 单元数据 (包含起始码)
   * @returns {{codec: string, width: number, height: number}}
   */
  static parseSPS(spsData) {
    // 跳过起始码 (4字节) 和 NAL header (1字节)
    if (spsData.length < 5) {
      throw new Error('Invalid SPS data');
    }

    const nalData = spsData.slice(4); // 去掉起始码

    // 提取 profile_idc, constraint_set_flags, level_idc
    const profile_idc = nalData[1];
    const constraint_set_flags = nalData[2];
    const level_idc = nalData[3];

    // 生成 codec 字符串 (格式: avc1.PPCCLL)
    const codec = `avc1.${this.toHex(profile_idc)}${this.toHex(constraint_set_flags)}${this.toHex(level_idc)}`;

    // 解析宽高需要完整的 SPS 解析, 这里使用简化方法
    // 从实际测试中, 大多数手机视频是 1080x2400 或类似分辨率
    // 完整解析需要 Exponential-Golomb 编码解码, 较复杂

    // 这里返回一个默认值, 实际宽高会在 VideoDecoder 配置后通过帧获取
    const width = 1080;  // 默认值, 会被实际解码覆盖
    const height = 2400; // 默认值, 会被实际解码覆盖

    return { codec, width, height };
  }

  /**
   * 将数字转为2位16进制字符串
   */
  static toHex(value) {
    return value.toString(16).padStart(2, '0').toUpperCase();
  }

  /**
   * 检查是否是关键帧 (IDR)
   */
  static isKeyFrame(data) {
    const type = this.getNaluType(data);
    return type === this.NALU_TYPE_IDR;
  }

  /**
   * 检查是否是 SPS
   */
  static isSPS(data) {
    const type = this.getNaluType(data);
    return type === this.NALU_TYPE_SPS;
  }

  /**
   * 检查是否是 PPS
   */
  static isPPS(data) {
    const type = this.getNaluType(data);
    return type === this.NALU_TYPE_PPS;
  }

  /**
   * 合并多个 Uint8Array
   */
  static combineNALUs(arrays) {
    let totalLength = 0;
    for (const arr of arrays) {
      totalLength += arr.length;
    }

    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const arr of arrays) {
      result.set(arr, offset);
      offset += arr.length;
    }

    return result;
  }

  /**
   * 拆分包含多个 NAL 单元的数据包
   * H.264 起始码: 00 00 00 01
   * @param {Uint8Array} data - 包含一个或多个 NAL 单元的数据
   * @returns {Array<Uint8Array>} NAL 单元数组
   */
  static splitNALUs(data) {
    const nalus = [];
    const startCode = new Uint8Array([0x00, 0x00, 0x00, 0x01]);

    let offset = 0;
    while (offset < data.length) {
      // 查找下一个起始码
      let nextStart = -1;
      for (let i = offset + 4; i <= data.length - 4; i++) {
        if (data[i] === 0x00 && data[i+1] === 0x00 && data[i+2] === 0x00 && data[i+3] === 0x01) {
          nextStart = i;
          break;
        }
      }

      // 提取当前 NAL 单元
      if (nextStart === -1) {
        // 最后一个 NAL 单元
        nalus.push(data.slice(offset));
        break;
      } else {
        nalus.push(data.slice(offset, nextStart));
        offset = nextStart;
      }
    }

    return nalus;
  }
}

// 导出到全局
window.ScrcpyH264Parser = H264Parser;
