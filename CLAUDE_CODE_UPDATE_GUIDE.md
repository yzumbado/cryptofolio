# How to Update Claude Code to Latest Version

## 📦 Update Methods

### **Method 1: Using npm (Recommended)**

If you installed Claude Code via npm:

```bash
# Update to latest version
npm install -g @anthropic-ai/claude-code@latest

# Verify new version
claude --version
```

### **Method 2: Using Homebrew (macOS/Linux)**

If you installed via Homebrew:

```bash
# Update Homebrew first
brew update

# Upgrade Claude Code
brew upgrade claude-code

# Verify
claude --version
```

### **Method 3: Direct Download**

If you installed from the website:

1. Visit: https://claude.ai/claude-code
2. Download the latest installer for your OS
3. Run the installer (it will update your existing installation)

### **Method 4: From Source**

If you cloned the repository:

```bash
cd ~/path/to/claude-code
git pull origin main
npm install
npm run build
```

---

## 🔍 Check Current Version

```bash
# Show current version
claude --version

# Show detailed version info
claude version --verbose

# Check for updates
claude update --check
```

---

## ⚙️ Update Command (If Available)

Some versions of Claude Code include a built-in update command:

```bash
# Check for and install updates
claude update

# Or with confirmation prompt
claude update --interactive
```

---

## 🔄 After Updating

### **1. Verify Installation**
```bash
claude --version
```

### **2. Check Configuration**
```bash
# Your settings are preserved across updates
claude config list
```

### **3. Test Functionality**
```bash
# Quick test
claude --help

# Or start a session
claude
```

---

## 🐛 Troubleshooting

### **Issue: Update Command Not Found**

```bash
# Uninstall old version
npm uninstall -g @anthropic-ai/claude-code

# Install latest
npm install -g @anthropic-ai/claude-code@latest
```

### **Issue: Permission Denied**

```bash
# Use sudo (not recommended, but works)
sudo npm install -g @anthropic-ai/claude-code@latest

# OR fix npm permissions (better approach)
mkdir ~/.npm-global
npm config set prefix '~/.npm-global'
echo 'export PATH=~/.npm-global/bin:$PATH' >> ~/.profile
source ~/.profile

# Then install without sudo
npm install -g @anthropic-ai/claude-code@latest
```

### **Issue: Old Version Still Showing**

```bash
# Clear npm cache
npm cache clean --force

# Reinstall
npm install -g @anthropic-ai/claude-code@latest

# Or check PATH
which claude
```

---

## 📋 Release Notes

Check what's new in the latest version:

- **Website**: https://claude.ai/claude-code/changelog
- **GitHub**: https://github.com/anthropics/claude-code/releases
- **In-app**: `claude changelog`

---

## 🔐 Preserve Your Settings

Your Claude Code settings are stored separately and won't be affected by updates:

**Settings Location:**
- macOS/Linux: `~/.claude/`
- Windows: `%USERPROFILE%\.claude\`

**What's Preserved:**
- API keys
- Project configurations
- Custom prompts
- Keyboard shortcuts
- Memory files

---

## ⚡ Quick Reference

| Task | Command |
|------|---------|
| Check version | `claude --version` |
| Update (npm) | `npm install -g @anthropic-ai/claude-code@latest` |
| Update (brew) | `brew upgrade claude-code` |
| Check for updates | `claude update --check` |
| View changelog | `claude changelog` |

---

## 🆕 What's New in Latest Versions

**Common improvements include:**
- Performance enhancements
- New tool integrations
- Bug fixes
- UI improvements
- Better error messages

Run `claude changelog` to see specific changes.

---

## 💡 Best Practices

1. **Update Regularly** - New versions include bug fixes and improvements
2. **Read Release Notes** - Know what changed before updating
3. **Test After Update** - Verify core functionality works
4. **Backup Projects** - Git commit your work before major updates
5. **Report Issues** - If something breaks, report it

---

## 📞 Need Help?

- **Documentation**: https://claude.ai/claude-code/docs
- **GitHub Issues**: https://github.com/anthropics/claude-code/issues
- **In-app**: `claude help`

---

**Current CI/CD Status:** CI should pass now with clippy errors fixed! ✅
