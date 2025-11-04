// WebCodecs 视频解码器 - 从 ws-scrcpy WebCodecsPlayer.ts 简化
// 负责H.264解码和Canvas渲染

class WebCodecsDecoder {
  constructor(canvas) {
    this.canvas = canvas;
    this.canvasContext = canvas.getContext('2d');
    this.decoder = null;
    this.isConfigured = false;

    // NAL 单元缓冲
    this.bufferedSPS = null;
    this.bufferedPPS = null;
    this.hadIDR = false;

    // 视频信息
    this.videoWidth = 0;
    this.videoHeight = 0;
    this.codec = '';

    // 解码帧队列 (用于平滑渲染)
    this.frameQueue = [];
    this.isRendering = false;

    // 首帧渲染标志
    this.hasRenderedFirstFrame = false;
    this.onFirstFrameCallback = null;
  }

  /**
   * 初始化解码器
   */
  init() {
    if (!window.VideoDecoder) {
      throw new Error('WebCodecs not supported');
    }

    this.decoder = new VideoDecoder({
      output: (frame) => this.onFrameDecoded(frame),
      error: (error) => this.onDecodeError(error)
    });

    window.rLog('✅ WebCodecs 解码器已初始化');
  }

  /**
   * 解码视频帧
   * @param {Uint8Array} data - H.264 NAL 单元数据 (可能包含多个 NAL 单元)
   */
  decode(data) {
    if (!data || data.length < 5) {
      window.rLog(`⚠️  数据太短，无法解码: ${data ? data.length : 0} 字节`);
      return;
    }

    const H264Parser = window.ScrcpyH264Parser;

    // 检查数据中是否包含多个 NAL 单元
    // 通过查找多个起始码 (00 00 00 01) 来判断
    let startCodeCount = 0;
    for (let i = 0; i <= data.length - 4; i++) {
      if (data[i] === 0x00 && data[i+1] === 0x00 && data[i+2] === 0x00 && data[i+3] === 0x01) {
        startCodeCount++;
      }
    }

    if (startCodeCount > 1) {
      // 包含多个 NAL 单元，需要拆分
      const nalus = H264Parser.splitNALUs(data);
      for (const nalu of nalus) {
        this.decodeNALU(nalu);
      }
      return;
    }

    // 只有一个 NAL 单元
    this.decodeNALU(data);
  }

  /**
   * 解码单个 NAL 单元
   * @param {Uint8Array} data - 单个 NAL 单元数据
   */
  decodeNALU(data) {
    if (!data || data.length < 5) {
      return;
    }

    const H264Parser = window.ScrcpyH264Parser;
    const naluType = H264Parser.getNaluType(data);

    // 处理 SPS
    if (naluType === H264Parser.NALU_TYPE_SPS) {
      window.rLog('✅ 收到 SPS (Sequence Parameter Set)');
      this.bufferedSPS = data;
      this.hadIDR = false;

      // 解析 SPS 获取 codec 和尺寸
      try {
        const { codec, width, height } = H264Parser.parseSPS(data);
        this.codec = codec;
        this.videoWidth = width;
        this.videoHeight = height;
        window.rLog(`📐 视频配置: ${width}x${height}, codec: ${codec}`);

        // 配置解码器
        this.configureDecoder();
      } catch (e) {
        window.rError('❌ 解析 SPS 失败:', e);
      }
      return;
    }

    // 处理 PPS
    if (naluType === H264Parser.NALU_TYPE_PPS) {
      window.rLog('✅ 收到 PPS (Picture Parameter Set)');
      this.bufferedPPS = data;
      return;
    }

    // 处理 SEI (忽略,除非有 SPS/PPS)
    if (naluType === H264Parser.NALU_TYPE_SEI) {
      if (!this.bufferedSPS || !this.bufferedPPS) {
        return; // 没有配置信息,丢弃
      }
    }

    // 检查是否是 IDR
    const isIDR = naluType === H264Parser.NALU_TYPE_IDR;
    this.hadIDR = this.hadIDR || isIDR;

    // 只有在有 SPS/PPS 且解码器已配置的情况下才解码
    if (!this.isConfigured || !this.hadIDR) {
      window.rLog(`跳过帧: isConfigured=${this.isConfigured}, hadIDR=${this.hadIDR}`);
      return;
    }

    // 对于 IDR 帧, 需要附加 SPS 和 PPS
    let frameData;
    if (isIDR && this.bufferedSPS && this.bufferedPPS) {
      window.rLog('收到 IDR 帧, 合并 SPS+PPS+IDR');
      frameData = H264Parser.combineNALUs([this.bufferedSPS, this.bufferedPPS, data]);
    } else {
      frameData = data;
    }

    // 创建 EncodedVideoChunk
    try {
      const chunk = new EncodedVideoChunk({
        type: isIDR ? 'key' : 'delta',
        timestamp: performance.now() * 1000, // 微秒
        data: frameData.buffer
      });

      this.decoder.decode(chunk);
    } catch (e) {
      window.rError('解码失败:', e);
    }
  }

  /**
   * 配置解码器
   */
  configureDecoder() {
    if (!this.decoder || !this.codec) {
      return;
    }

    const config = {
      codec: this.codec,
      optimizeForLatency: true
    };

    try {
      this.decoder.configure(config);
      this.isConfigured = true;
      window.rLog('✅ 解码器已配置:', config);
    } catch (e) {
      window.rError('配置解码器失败:', e);
    }
  }

  /**
   * 解码回调: 收到解码后的帧
   */
  onFrameDecoded(frame) {
    // 更新视频尺寸 (从实际帧获取)
    if (frame.displayWidth && frame.displayHeight) {
      if (this.videoWidth !== frame.displayWidth || this.videoHeight !== frame.displayHeight) {
        this.videoWidth = frame.displayWidth;
        this.videoHeight = frame.displayHeight;
        window.rLog(`🖼️  更新视频尺寸: ${this.videoWidth}x${this.videoHeight}`);

        // 调整 Canvas 尺寸 (内部渲染尺寸)
        this.canvas.width = this.videoWidth;
        this.canvas.height = this.videoHeight;

        // 显示 Canvas 的 CSS 尺寸（用于调试）
        const rect = this.canvas.getBoundingClientRect();
        window.rLog(`📐 Canvas CSS 尺寸: ${rect.width.toFixed(0)}x${rect.height.toFixed(0)}`);
      }
    }

    // 直接渲染到 Canvas
    try {
      this.canvasContext.drawImage(frame, 0, 0);

      // 检测首帧渲染
      if (!this.hasRenderedFirstFrame) {
        this.hasRenderedFirstFrame = true;
        window.rLog('🎬 首帧已渲染');

        // 触发首帧回调
        if (this.onFirstFrameCallback) {
          this.onFirstFrameCallback();
          this.onFirstFrameCallback = null; // 只调用一次
        }
      }
    } catch (e) {
      window.rError('渲染帧失败:', e);
    } finally {
      frame.close(); // 释放帧资源
    }
  }

  /**
   * 解码错误回调
   */
  onDecodeError(error) {
    window.rError('WebCodecs 解码错误:', error);
    this.isConfigured = false;
  }

  /**
   * 设置首帧渲染回调
   * @param {Function} callback - 首帧渲染后调用的回调函数
   */
  setOnFirstFrameCallback(callback) {
    this.onFirstFrameCallback = callback;
  }

  /**
   * 停止解码器
   */
  stop() {
    if (this.decoder && this.decoder.state === 'configured') {
      this.decoder.close();
    }
    this.decoder = null;
    this.isConfigured = false;
    this.bufferedSPS = null;
    this.bufferedPPS = null;
    this.hadIDR = false;
    this.hasRenderedFirstFrame = false;
    this.onFirstFrameCallback = null;
  }

  /**
   * 获取视频尺寸
   */
  getVideoSize() {
    return {
      width: this.videoWidth,
      height: this.videoHeight
    };
  }
}

// 导出到全局
window.ScrcpyWebCodecsDecoder = WebCodecsDecoder;
