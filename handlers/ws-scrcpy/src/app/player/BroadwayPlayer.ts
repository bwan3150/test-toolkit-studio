import '../../../vendor/Broadway/avc.wasm.asset';
import { BaseCanvasBasedPlayer } from './BaseCanvasBasedPlayer';
import Size from '../Size';
import YUVCanvas from '../../../vendor/h264-live-player/YUVCanvas';
import YUVWebGLCanvas from '../../../vendor/h264-live-player/YUVWebGLCanvas';
import Avc from '../../../vendor/Broadway/Decoder';
import VideoSettings from '../VideoSettings';
import Canvas from '../../../vendor/h264-live-player/Canvas';
import { DisplayInfo } from '../DisplayInfo';

export class BroadwayPlayer extends BaseCanvasBasedPlayer {
    public static readonly storageKeyPrefix = 'BroadwayDecoder';
    public static readonly playerFullName = 'Broadway.js';
    public static readonly playerCodeName = 'broadway';
    public static readonly preferredVideoSettings: VideoSettings = new VideoSettings({
        lockedVideoOrientation: -1,
        bitrate: 524288,
        maxFps: 24,
        iFrameInterval: 5,
        bounds: new Size(480, 480),
        sendFrameMeta: false,
    });

    protected canvas?: Canvas;
    private avc?: Avc;
    public readonly supportsScreenshot: boolean = true;

    public static isSupported(): boolean {
        return typeof WebAssembly === 'object' && typeof WebAssembly.instantiate === 'function';
    }

    constructor(udid: string, displayInfo?: DisplayInfo, name = BroadwayPlayer.playerFullName) {
        super(udid, displayInfo, name, BroadwayPlayer.storageKeyPrefix);
    }

    protected initCanvas(width: number, height: number): void {
        console.log('[BroadwayPlayer] 🎨 初始化 Canvas, 尺寸:', width, 'x', height);
        super.initCanvas(width, height);
        if (BaseCanvasBasedPlayer.hasWebGLSupport()) {
            console.log('[BroadwayPlayer] ✅ 使用 WebGL Canvas');
            this.canvas = new YUVWebGLCanvas(this.tag, new Size(width, height));
        } else {
            console.log('[BroadwayPlayer] ⚠️  使用 2D Canvas (无 WebGL 支持)');
            this.canvas = new YUVCanvas(this.tag, new Size(width, height));
        }
        if (!this.avc) {
            console.log('[BroadwayPlayer] 🚀 创建 Broadway 解码器');
            this.avc = new Avc();
        }
        this.avc.onPictureDecoded = (buffer: Uint8Array, width: number, height: number) => {
            console.log('[BroadwayPlayer] 🖼️  解码完成一帧:', width, 'x', height);
            this.onFrameDecoded(width, height, buffer);
        };
    }

    protected decode(data: Uint8Array): void {
        if (!this.avc) {
            console.log('[BroadwayPlayer] ⚠️  解码器未初始化');
            return;
        }
        console.log('[BroadwayPlayer] 🎬 解码视频数据:', data.length, '字节');
        this.avc.decode(data);
    }

    public getPreferredVideoSetting(): VideoSettings {
        return BroadwayPlayer.preferredVideoSettings;
    }

    public getFitToScreenStatus(): boolean {
        return BroadwayPlayer.getFitToScreenStatus(this.udid, this.displayInfo);
    }

    public loadVideoSettings(): VideoSettings {
        return BroadwayPlayer.loadVideoSettings(this.udid, this.displayInfo);
    }
}
