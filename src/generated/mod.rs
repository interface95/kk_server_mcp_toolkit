// 直接包含预生成的 protobuf 代码，无需 build.rs
pub mod com {
    pub mod kuaishou {
        pub mod protobuf {
            pub mod log {
                include!("com.kuaishou.protobuf.log.rs");
            }
        }

        pub mod client {
            pub mod log {
                include!("com.kuaishou.client.log.rs");

                pub mod content {
                    include!("com.kuaishou.client.log.content.rs");
                }

                pub mod event {
                    include!("com.kuaishou.client.log.event.rs");
                }

                pub mod stat {
                    include!("com.kuaishou.client.log.stat.rs");
                }

                pub mod task {
                    pub mod detail {
                        include!("com.kuaishou.client.log.task.detail.rs");
                    }
                }
            }
        }
    }
}
