// 【用量累计】一次安全测试烧了多少 token —— 平台按它计费（ADR-0023 D3）。
//
// 为什么安全轨要单独有这么一件：harness 那边有 `Summary` 事件带「全程总量」，
// 安全轨没有（它无头跑完只打一个结果对象），于是任务层拿不到用量、平台只能计设备时长。
//
// **没测到就是没测到**：`is_measured()` 为假时，上层要给 null 而不是 0——
// 0 会被平台读成"这次没花钱"，而真相是"没量到"（INV-9：查不了要说出来）。
//
// 分角色留一份（prober / analyst / orchestrator）：钱花在自主探测还是对抗复核上，
// 是能指导调优的信息，而合并成一个总数就再也分不开了。

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RoleUsage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    /// 这个角色发起了几次会话（analyst 是每条 finding 一次）
    pub calls: u32,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Usage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    /// 合计。**序列化出去**，省得每个消费方自己加一遍（加错了没人发现）
    pub total_tokens: i64,
    /// 最后一次调用用的模型。多模型混跑时它只是个提示，精确账看 by_role
    pub model: String,
    pub by_role: BTreeMap<String, RoleUsage>,
}

impl Usage {
    /// 记一次调用的用量
    pub fn add(&mut self, role: &str, model: &str, prompt: i64, completion: i64) {
        let e = self.by_role.entry(role.to_string()).or_default();
        e.prompt_tokens += prompt;
        e.completion_tokens += completion;
        e.calls += 1;
        self.prompt_tokens += prompt;
        self.completion_tokens += completion;
        self.total_tokens = self.prompt_tokens + self.completion_tokens;
        if !model.is_empty() {
            self.model = model.to_string();
        }
    }

    /// 并进另一份（prober 的 + analyst 的）
    pub fn merge(&mut self, other: &Usage) {
        for (role, u) in &other.by_role {
            let e = self.by_role.entry(role.clone()).or_default();
            e.prompt_tokens += u.prompt_tokens;
            e.completion_tokens += u.completion_tokens;
            e.calls += u.calls;
        }
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.total_tokens = self.prompt_tokens + self.completion_tokens;
        if !other.model.is_empty() {
            self.model = other.model.clone();
        }
    }

    /// **量到了没有**。一次调用都没记过 = 没量到（供应商没回 usage、或者根本没调 AI），
    /// 与"调了但真的是 0"区分不开时，宁可报没量到
    pub fn is_measured(&self) -> bool {
        !self.by_role.is_empty() && self.total_tokens > 0
    }

    /// 给平台的形态：**没量到就是 null**，不是一堆 0
    pub fn to_json(&self) -> serde_json::Value {
        if self.is_measured() {
            serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Null
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 分角色记账并给出合计() {
        let mut u = Usage::default();
        u.add("prober", "m1", 100, 20);
        u.add("analyst", "m1", 50, 10);
        u.add("analyst", "m1", 50, 10);
        assert_eq!(u.prompt_tokens, 200);
        assert_eq!(u.completion_tokens, 40);
        assert_eq!(u.total_tokens, 240, "合计要自己算好——让每个消费方各加一遍迟早有人加错");
        assert_eq!(u.by_role["analyst"].calls, 2, "analyst 是每条 finding 一次，次数本身有意义");
        assert_eq!(u.by_role["prober"].prompt_tokens, 100);
    }

    #[test]
    fn 合并两份() {
        let mut a = Usage::default();
        a.add("prober", "m", 100, 20);
        let mut b = Usage::default();
        b.add("analyst", "m", 30, 5);
        b.add("prober", "m", 1, 1);
        a.merge(&b);
        assert_eq!(a.total_tokens, 157, "prompt 100+30+1=131, completion 20+5+1=26");
        assert_eq!(a.by_role["prober"].calls, 2);
        assert_eq!(a.by_role["analyst"].calls, 1);
    }

    #[test]
    fn 没量到就给null不给一堆零() {
        // 0 会被平台读成"这次没花钱"，而真相是"没量到"
        let empty = Usage::default();
        assert!(!empty.is_measured());
        assert!(empty.to_json().is_null());

        // 调了但供应商没回 usage（全 0）——同样算没量到，宁可报不知道
        let mut zero = Usage::default();
        zero.add("prober", "m", 0, 0);
        assert!(!zero.is_measured());
        assert!(zero.to_json().is_null());

        let mut real = Usage::default();
        real.add("prober", "m", 1, 1);
        assert!(real.is_measured());
        assert_eq!(real.to_json()["total_tokens"], 2);
    }
}
