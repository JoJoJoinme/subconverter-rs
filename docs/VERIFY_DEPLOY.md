# 验证 Cloudflare Workers 自动部署流程

本文档旨在帮助您验证 GitHub Actions 自动部署流程是否正常工作。

## 1. 准备工作

确保您拥有一个 Cloudflare 账户，并且已经获取了以下信息：
- **API Token**: 具有 Workers 编辑权限的 API 令牌。
- **Account ID**: 您的 Cloudflare 账户 ID。

## 2. Fork 仓库

如果您还没有 Fork 本仓库，请点击页面右上角的 "Fork" 按钮。

## 3. 配置 Secrets

在您的 Fork 仓库中：
1.  点击 **Settings** (设置)。
2.  在左侧侧边栏中选择 **Secrets and variables** -> **Actions**。
3.  点击 **New repository secret**。
4.  添加 `CLOUDFLARE_API_TOKEN`，填入您的 API Token。
5.  点击 **New repository secret**。
6.  添加 `CLOUDFLARE_ACCOUNT_ID`，填入您的 Account ID。

## 4. 修改 KV ID (可选)

为了确保部署成功，您需要在 `cloudflare/wrangler.toml` 中配置正确的 KV Namespace ID。
如果只是验证部署流程是否跑通（不运行实际业务），您可以暂时跳过此步，但 Workers 启动后可能会报错。

建议步骤：
1.  在本地或 Cloudflare 仪表盘创建一个 KV Namespace。
2.  修改 `cloudflare/wrangler.toml`:
    ```toml
    [[kv_namespaces]]
    binding = "KV"
    id = "您的_KV_ID"
    ```
3.  提交更改。

> **关于安全性:**
> 您可能担心 KV ID 是否可以暴露。KV Namespace ID 本身是一个公开的标识符，类似于 UUID。仅凭 KV ID，他人**无法**读取或写入您的数据。必须配合具有相应权限的 **API Token** 和 **Account ID** 才能进行操作。因此，将 ID 提交到代码仓库（`wrangler.toml`）是安全的，但请务必保护好您的 Secrets。

## 5. 触发部署

有两种方式触发部署：

### 方式 A: 手动触发 (Workflow Dispatch)
1.  点击仓库上方的 **Actions** 选项卡。
2.  在左侧选择 **Deploy to Cloudflare Workers**。
3.  点击右侧的 **Run workflow** 按钮。
4.  选择 `main` 分支，点击绿色按钮确认。

### 方式 B: 推送触发
1.  修改仓库中的任意文件（例如 `README.md` 添加一个空格）。
2.  提交并推送到 `main` 分支。

## 6. 验证结果

1.  在 **Actions** 页面观察工作流运行状态。
2.  等待工作流变为绿色（Success）。
3.  点击工作流运行记录，展开 `Deploy to Cloudflare Workers` 步骤。
4.  在日志末尾，您应该能看到 Wrangler 输出的部署 URL，例如：
    ```
    Published subconverter-worker (4.04 sec)
      https://subconverter-worker.<您的子域名>.workers.dev
    ```
5.  访问该 URL，如果看到 `Subconverter is running!` 或类似提示，说明部署成功。
