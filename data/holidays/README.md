# 中国大陆节假日数据

日历内置 `china.json`，联网只请求本仓库 main 分支的 `manifest.json`；版本变新时才下载 `china.json`。客户端默认关闭自动联网，手动检查不受每日间隔限制。缓存保存于本机 SQLite；失败不清空缓存。公开更新地址须在这两个文件推送到 main 后才可使用。

## 发布步骤

1. 根据国务院年度放假通知核对放假区间及调休上班日，每个年份记录官方 HTTPS 来源。
2. 修改 `china.json`：每个放假日为 `holiday`，每个调休上班日为 `workday`；普通周末不记录。每个年份保存完整安排，修改已有年份时也必须保留该年份其余日期。
3. 更新 `updatedAt` 为数据核对日期，增加整数 `revision`（推荐 YYYYMMDDNN），并让 `manifest.json` 使用相同版本。
4. 运行 `npm test`、`cargo test --lib`（在 src-tauri 目录），检查重复日期、年份和版本；两个文件放在同一提交中发布到 main。

当前数据依据 2025-11-04 公布的 2026 年安排，来源：https://www.beijing.gov.cn/fuwu/bmfw/sy/jrts/202511/t20251104_4258838.html 。

## 格式

- `schemaVersion` 当前为 1；不兼容格式须升级客户端。
- `revision` 必须单调递增，最大为 JavaScript 安全整数 9007199254740991。
- `years` 以年份区分；客户端合并新年份并保留已缓存的旧年份。
- `days` 日期使用 YYYY-MM-DD、名称最多 24 字、类型仅为 `holiday` 或 `workday`。
- 单个文件不超过 1 MiB，来源必须为 HTTPS 的 gov.cn 或其子域名。
- 未收录的年份显示“假期安排尚未收录”，不根据农历推算官方放假或调休。

在线数据的正确性由项目维护者核对；客户端校验格式、日期及版本，不能代替官方内容审核。
