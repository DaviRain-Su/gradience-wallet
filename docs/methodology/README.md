# Dev Lifecycle 方法论

## 来源

本项目遵循 [dev-lifecycle](https://codeberg.org/davirain/dev-lifecycle) 方法论。

## 核心规则

1. **不可跳过阶段** — PRD → Architecture → Technical Spec → Task Breakdown → Test Spec → Implementation → Review
2. **技术规格是代码的契约** — 代码必须与规格 100% 一致
3. **TDD 不可商量** — 测试先于实现
4. **输入完整才能开始** — 上一阶段输出是下一阶段输入
5. **必填项不可省略**

## 文档结构

```
docs/
├── methodologoy/README.md          ← 本文件（方法论引用）
├── 01-prd.md                       ← Phase 1
├── 02-architecture.md              ← Phase 2
├── 03-technical-spec.md            ← Phase 3（最重要）
├── 04-task-breakdown.md            ← Phase 4
├── 05-test-spec.md                ← Phase 5
├── 06-implementation-log.md       ← Phase 6
└── 07-review-report.md            ← Phase 7
```

## 完整模板

完整模板仓库：https://codeberg.org/davirain/dev-lifecycle
