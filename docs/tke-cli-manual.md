# TKE命令行指令与输出示例

## tke adb

- 将所有`adb`能用的指令前面加上`tke`就可以通过tke, 直接访问内嵌的adb执行原生adb指令
- 例子:
```bash
tke adb --version

Android Debug Bridge version 1.0.41
Version 36.0.0-13206524
Installed as /var/folders/pq/l00t670n1fs_74g7bt9m460r0000gn/T/tke_adb/adb
Running on Darwin 24.6.0 (arm64)
```

## tke ocr 

- http请求在线的OCR服务API
```bash
❯ tke ocr --image /path/to/test/phone/screenshot.png --online --url https://ocr.test-toolkit.app/ocr

{"texts":[{"text":"SOs46","bbox":[[654.0,33.0],[1010.0,33.0],[1010.0,83.0],[654.0,83.0]],"confidence":0.63283575},{"text":"P:0/1","bbox":[[0.0,93.0],[99.0,93.0],[99.0,131.0],[0.0,131.0]],"confidence":0.98180187},{"text":"dX: 95.4 dY:872.7Xv: 3.553 Yv:9.755","bbox":[[151.0,93.0],[750.0,93.0],[750.0,128.0],[151.0,128.0]],"confidence":0.9625972},{"text":"Prs:1.0","bbox":[[763.0,93.0],[888.0,93.0],[888.0,131.0],[763.0,131.0]],"confidence":0.9885947},{"text":"Size: 0.02","bbox":[[916.0,90.0],[1064.0,90.0],[1064.0,133.0],[916.0,133.0]],"confidence":0.97435606},{"text":"1:33 Mon, 29 Sept","bbox":[[76.0,123.0],[519.0,128.0],[519.0,181.0],[75.0,176.0]],"confidence":0.96730155},{"text":"Konect..","bbox":[[205.0,324.0],[408.0,324.0],[408.0,377.0],[205.0,377.0]],"confidence":0.9957355},{"text":"Blueto..","bbox":[[210.0,559.0],[403.0,569.0],[400.0,634.0],[207.0,625.0]],"confidence":0.97263926},{"text":"● Arks·Now ","bbox":[[96.0,1053.0],[444.0,1053.0],[444.0,1103.0],[96.0,1103.0]],"confidence":0.9120194},{"text":"Mtion Alarm","bbox":[[84.0,1120.0],[377.0,1126.0],[376.0,1186.0],[83.0,1181.0]],"confidence":0.99745584},{"text":"Motion Sensor Motion detected.","bbox":[[88.0,1201.0],[805.0,1201.0],[805.0,1251.0],[88.0,1251.0]],"confidence":0.9851459},{"text":"Silent","bbox":[[52.0,1364.0],[236.0,1364.0],[236.0,1437.0],[52.0,1437.0]],"confidence":0.9989075},{"text":"HID Mobile Access·Now","bbox":[[81.0,1535.0],[647.0,1538.0],[646.0,1588.0],[80.0,1585.0]],"confidence":0.9397481},{"text":"Y","bbox":[[927.0,1555.0],[945.0,1555.0],[945.0,1568.0],[927.0,1568.0]],"confidence":0.5344238},{"text":"HID Mobile Access running in","bbox":[[83.0,1605.0],[748.0,1611.0],[747.0,1671.0],[83.0,1666.0]],"confidence":0.99605423},{"text":"the background","bbox":[[86.0,1666.0],[452.0,1666.0],[452.0,1726.0],[86.0,1726.0]],"confidence":0.99765337},{"text":"Tap the icon to use your Mobil..","bbox":[[86.0,1741.0],[766.0,1741.0],[766.0,1799.0],[86.0,1799.0]],"confidence":0.98489875}]}
```

请求本地内封tesseract
```bash
tke ocr --image /path/to/test/phone/screenshot.png --lang eng

尚未解决训练集问题, 本地的暂时无法使用, 先都用online
```
