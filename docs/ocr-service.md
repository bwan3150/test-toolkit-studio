# OCR图像文字提取接口文档

## 接口

- POST {{OCR_HOST}}/ocr

headers:
Content-Type : application/json

body示例: 

```json

{"image":"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"(将要进行文字内容提取的图片先转为BASE64格式再放到这里) 

```

返回示例:

```json

{
    "processing_time": 0.05233407020568848,
    "results": [
        {
            "bounding_box": [
                [
                    8.0,
                    5.0
                ],
                [
                    29.0,
                    7.0
                ],
                [
                    28.0,
                    16.0
                ],
                [
                    7.0,
                    13.0
                ]
            ],
            "confidence": 0.7511634826660156,
            "text": "Ceet"
        }
    ]
}

```


## 注意事项

1. 需要先将图片转化为BASE64格式, 然后放到body json的image字段  
2. 返回为处理时间, 文字内容, 文字内容对应的bonding box, 文字内容对应的识别置信度  
3. 这里返回的所有信息, 需要对应渲染在原图对应位置, 比如框出识别出的文字的bonding box标注置信度, 用户可以直接在图上将文字复制走  
4. 用户可以选择文字框可见或不可见, 不影响文字内容的复制, 不过默认开启文字内容框可见  
