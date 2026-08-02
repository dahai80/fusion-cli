# Phase 1 基础重构实施计划

## 目标
消灭死代码，统一服务连接，真实实现核心功能，零警告编译

## 变更清单

### Step 1: 创建统一服务层 `src/service/`
- 新建 `src/service/mod.rs` — ServiceRegistry + 全局 reqwest::Client
- 新建 `src/service/mlx.rs` — 从 mlx_bind/ 迁移，增加 SSE 流式支持
- 新建 `src/service/kb.rs` — 从 ecosystem/knowledge_base 迁移
- 新建 `src/service/modelhub.rs` — 从 ecosystem/model_hub 迁移
- 新建 `src/service/desk.rs` — 从 ecosystem/desk 迁移
- 新建 `src/service/rag.rs` — RAG 客户端
- 新建 `src/service/health.rs` — 统一健康检查
- 服务 URL 统一从 config.toml 读取，带 env override

### Step 2: 增强配置体系 `src/config/mod.rs`
- FusionConfig 新增字段：service URLs, cache_size, max_batch_size
- 所有新字段有 Default，向后兼容 V0.1
- config get/set 支持所有新 key

### Step 3: 删除死模块
- 删除 ecosystem/, mlx_bind/, system/
- main.rs 更新 mod 声明

### Step 4: 重构 cmd/ 使用 service 层
- chat.rs — 使用 service::mlx + SSE 流式
- service.rs — 真实启停实现
- model.rs — pull 从 ModelHub 真实下载
- bench.rs — 真实评测
- kb.rs, doctor.rs, version.rs, sync.rs, cluster.rs — 使用 service 层

### Step 5: 全局 reqwest::Client 连接池

### Step 6: 清理 Cargo.toml 未使用依赖

### Step 7: 验证零警告编译
