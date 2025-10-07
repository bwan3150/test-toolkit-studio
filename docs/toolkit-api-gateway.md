# Toolkit主API网关文档

## 功能

这是包括手机端(Portable Toolkit), 桌面端(Toolkit Desktop), 工作室(Toolkit Studio) 以及 Web端(Online Toolkit) 共用的账号认证和基础功能API网关

## 接口

### 账号认证模块

- 登录获取Access Token

POST {{BASE_URL}}/api/auth/login



Headers:

Content-Type : application/x-www-form-urlencoded



Body(x-www-form-urlencoded):

email : xxxxx@konec.com.au

password : xxxxxxxx



返回类似:

```json
{
    "access_token": "sdfsdfjdhglfkashdlfkjhalskdjhflk",
    "token_type": "bearer",
    "expires_in": 3600,
    "refresh_token": "askldfhlakjshdflkjahsdlfkhjalskjdf"
}
```



-   刷新Access Token

POST {{BASE_URL}}/api/auth/refresh



Headers:

Content-Type : application/json



Body:

```json
{
  "refresh_token": "askldfhlakjshdflkjahsdlfkhjalskjdf"
}
```



返回类似:

```json
{
    "access_token": "ziozizisofhjapzsfjkopasjfpo",
    "token_type": "bearer",
    "expires_in": 3600,
    "refresh_token": "asdpoiajspdjaposjdpaosjdpoajsp"
}
```





### 账号信息模块

-   获取当前登录用户信息

GET {{BASE_URL}}/api/users/me



Headers:

带着Beaerer access_token请求, 就会返回对应用户的信息了



返回类似:

```json
{
    "success": true,
    "user": {
        "_id": "xxxxxxixixixix",
        "username": "xxxxx",
        "email": "xxxxxxxxx@konec.com.au",
        "memberGroup": "Tester",
        "avatar": 1,
        "createdAt": "2025-01-01T01:24:10.109Z",
        "updatedAt": "2025-02-02T16:39:48.441Z",
        "__v": 0
    }
}
```





### 账号注册模块

-   新注册用户验证码发送

POST {{BASE_URL}}/api/users/register/send-code



Headers:

Content-Type : application/json



Body:

```json
{
  "email": "xxxxxxxx@konec.com.au"
}
```



返回类似:

```json
{
    "success": true,
    "message": "验证码已发送，请查收邮件"
}
```



-   注册新用户账号

POST {{BASE_URL}}/api/users/register



Headers:

Content-Type : application/json



Body:

```json
{
  "username": "New User",
  "email": "xxxxxxxx@konec.com.au",
  "password": "xxxxaxaxxxaxa",
  "verificationCode": "667576"
}
```



返回类似:

```json
{
    "success": true,
    "message": "用户注册成功，等待管理员分配成员组",
    "user": {
        "username": "New User",
        "email": "xxxxxxxx@konec.com.au",
        "memberGroup": "None",
        "avatar": 0,
        "_id": "isisisisisisisiifsdiasfa",
        "createdAt": "2025-09-17T05:22:21.547Z",
        "updatedAt": "2025-09-17T05:22:21.547Z",
        "__v": 0
    }
}
```

