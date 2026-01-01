# ClamAV Complete System Scan Script

A comprehensive Bash script for performing full system virus scans using ClamAV antivirus. This script automates the entire scanning process with progress tracking, logging, and system safety features.

> **Disclaimer**: This is a third-party automation script for ClamAV. ClamAV is an open-source antivirus engine owned by Cisco Systems, Inc. This project is not affiliated with, endorsed by, or connected to Cisco Systems or the official ClamAV project.

## Features

### 🛡️ Security & Safety
- **Root Privilege Check**: Ensures script runs with appropriate permissions
- **Service Management**: Automatically stops/stops freshclam service to prevent conflicts
- **Directory Exclusion**: Skips system directories like `/proc`, `/sys`, `/dev` to avoid issues
- **Error Handling**: Comprehensive error checking and graceful failure handling

### 📊 Progress Tracking
- **Real-time Progress Bar**: Visual progress indicator showing scan completion percentage
- **File Counting**: Pre-scans to estimate total files for accurate progress tracking
- **Live Updates**: Progress updates every 100 files scanned

### 📝 Logging & Reporting
- **Timestamped Logs**: Automatic log file generation with date/time stamps
- **Dual Output**: Both console display and file logging
- **Scan Reports**: Detailed scan results saved separately
- **Summary Display**: Quick overview of scan results at completion

### 🔄 Automation Features
- **Virus Database Updates**: Automatic freshclam execution before scanning
- **Database Age Check**: Warns if virus definitions are outdated (>7 days)
- **Infected File Handling**: Automatic detection and removal of viruses
- **Exit Code Interpretation**: Clear explanations of ClamAV exit codes

## Technical Details

### Scan Configuration
- **Recursive Scanning**: Scans all subdirectories
- **Archive Scanning**: Examines compressed files
- **PUA Detection**: Potentially Unwanted Applications detection
- **File Size Limits**: 512MB maximum per file/archive
- **Excluded Directories**: `/proc`, `/sys`, `/dev`, `/snap`, `/run`, `/tmp`

### Output Files
- **Log File**: `/var/log/clamav-full-scan-YYYYMMDD-HHMMSS.log`
- **Scan Report**: `/var/log/clamav-scan-report-YYYYMMDD-HHMMSS.txt`

## Prerequisites

### Required Software
- **ClamAV**: Must be installed on your system
- **freshclam**: ClamAV's virus definition update tool
- **Bash**: Version 4.0 or higher

### Installation
```bash
# Install ClamAV on Ubuntu/Debian
sudo apt-get update
sudo apt-get install clamav clamav-daemon

# Install ClamAV on CentOS/RHEL
sudo yum install clamav clamav-update

# Make script executable
    chmod +x full-system-scan.sh

# Run as root
    sudo ./full-system-scan.sh

Exit Codes Interpretation

0: No viruses found
1: Virus(es) found and removed
2: Some files could not be scanned
 Other: Unknown error occurred

Dependencies

-ClamAV: Must be installed (clamscan and freshclam commands)
-Systemd: For service management (freshclam)
-Root Access: Required for system-wide scanning

Safety Notes

-The script automatically handles freshclam service to prevent conflicts
-System directories are excluded to avoid scanning special filesystems
-Large files (>512MB) are skipped to prevent memory issues
-All actions are logged for audit purposes

Performance

-Initial File Count: May take time on large systems
-Scan Speed: Depends on system size and disk speed
-Progress Updates: Minimal overhead from progress tracking

Ideal For
-Regular system security audits
-Post-intrusion verification
-Scheduled security scanning
-System administrator toolkits

This script provides an enterprise-grade scanning solution with user-friendly features while maintaining ClamAV's powerful detection capabilities.
