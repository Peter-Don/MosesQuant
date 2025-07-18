#!/bin/bash

# MosesQuant GitHub 推送脚本
# 用于将本地代码推送到 GitHub 仓库

echo "=== MosesQuant GitHub 推送脚本 ==="
echo "目标仓库: https://github.com/Peter-Don/MosesQuant.git"
echo ""

# 检查当前分支
echo "当前分支:"
git branch --show-current

echo ""
echo "检查待推送的提交:"
git log --oneline -3

echo ""
echo "推送到 GitHub..."

# 尝试推送到 GitHub
if git push origin master; then
    echo "✅ 推送成功！"
    echo "🎉 MosesQuant Python FFI 绑定已成功推送到 GitHub"
    echo ""
    echo "🔗 查看仓库: https://github.com/Peter-Don/MosesQuant"
else
    echo "❌ 推送失败"
    echo ""
    echo "🔧 可能的解决方案:"
    echo "1. 检查网络连接"
    echo "2. 确认 GitHub 访问权限"
    echo "3. 检查代理设置"
    echo "4. 尝试使用 SSH 而非 HTTPS:"
    echo "   git remote set-url origin git@github.com:Peter-Don/MosesQuant.git"
    echo "   git push origin master"
    echo ""
    echo "5. 如果仍然失败，请手动上传代码到 GitHub"
fi

echo ""
echo "=== 推送完成 ==="