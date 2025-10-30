package app.TestToolkit.TKE.AutoServer.video;

import app.TestToolkit.TKE.AutoServer.control.PositionMapper;

public interface VirtualDisplayListener {
    void onNewVirtualDisplay(int displayId, PositionMapper positionMapper);
}
