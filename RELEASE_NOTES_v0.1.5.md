# Release Notes - v0.1.5

## 🎯 Overview

This release focuses on CI/CD reliability improvements, security enhancements, and documentation updates. All CI pipelines are now fully functional with comprehensive testing across multiple platforms.

## ✨ Key Improvements

### CI/CD Enhancements
- **Fixed CI test failures** by installing Terraform in GitHub Actions check job
- **Enhanced environment detection** for MCP integration tests with multiple fallback mechanisms
- **Improved CI reliability** with environment-specific test strategies
- **Documented CI/CD best practices** including troubleshooting guides

### Security & Code Quality
- **Removed all mock code** and mock frameworks for enhanced security
- **Enforced strict quality standards** with no dead code allowed
- **Added comprehensive security rules** documented in CLAUDE.md
- **Temporarily disabled cargo-audit** due to toolchain compatibility (requires Rust 1.85+)

### Documentation Updates
- **Completely reorganized CLAUDE.md** for better readability and maintenance
- **Added CI/CD troubleshooting section** with detailed problem-solution pairs
- **Documented known issues** and their resolutions with timestamps
- **Removed obsolete files** including rules/, package.json, and unused scripts

## 🔧 Technical Details

### Files Modified
- `.github/workflows/rust.yml` - Added Terraform installation and security audit workaround
- `tests/mcp_integration.rs` - Enhanced CI environment detection
- `CLAUDE.md` - Comprehensive reorganization and CI/CD documentation
- Removed multiple unused Rust modules and dependencies

### Testing
- ✅ All tests passing locally and in CI
- ✅ Cross-platform testing (Ubuntu, Windows, macOS)
- ✅ Code formatting and linting checks
- ✅ Code coverage reporting

## 📝 Breaking Changes

None - This release maintains full backward compatibility.

## 🚀 Upgrading

```bash
# Using cargo
cargo install tfmcp --version 0.1.5

# From source
git clone https://github.com/nwiizo/tfmcp
cd tfmcp
cargo install --path .
```

## 🔍 Known Issues

- Security audit temporarily disabled in CI due to cargo-audit requiring Rust 1.85+ (edition2024)
- Manual security review required until toolchain compatibility is resolved

## 🙏 Acknowledgments

Thanks to all contributors for helping improve the reliability and security of tfmcp!