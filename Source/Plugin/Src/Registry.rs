#![allow(non_snake_case)]

use lazy_static::lazy_static;
use std::sync::{RwLock, Weak};
use std::collections::HashMap;
use crossbeam::channel::Sender;
use crate::mcm_info;

/// 全局插件注册表
///
/// 用于管理所有活跃的插件实例，实现 Master-Slave 通信
pub struct GlobalRegistry {
    // 存储所有实例的消息发送端 (弱引用)
    // Key: 实例 ID (uuid), Value: Weak Sender
    instances: HashMap<String, Weak<Sender<()>>>,
}

lazy_static! {
    static ref REGISTRY: RwLock<GlobalRegistry> = RwLock::new(GlobalRegistry {
        instances: HashMap::new(),
    });
}

impl GlobalRegistry {
    pub fn register_instance() {
        // TODO: 注册逻辑
        mcm_info!("Plugin instance registered to GlobalRegistry");
    }
}

