#!/bin/bash

# ClamAV Complete System Scan Script for TUI
# Save as: clam.sh

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Log files
LOG_FILE="/var/log/clamav-full-scan-$(date +%Y%m%d-%H%M%S).log"
SCAN_REPORT="/var/log/clamav-scan-report-$(date +%Y%m%d-%H%M%S).txt"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: This script must be run as root (use sudo)${NC}"
    exit 1
fi

# Function to log messages 
log_message() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" | tee -a "$LOG_FILE"
}

# Header
echo "==========================================" | tee -a "$LOG_FILE"
echo "ClamAV Full System Scan Started" | tee -a "$LOG_FILE"
echo "Date: $(date)" | tee -a "$LOG_FILE"
echo "Log: $LOG_FILE" | tee -a "$LOG_FILE"
echo "Report: $SCAN_REPORT" | tee -a "$LOG_FILE"
echo "==========================================" | tee -a "$LOG_FILE"

# Step 1: Check if ClamAV is installed
log_message "Checking ClamAV installation..."
if ! command -v clamscan &> /dev/null; then
    echo -e "${RED}Error: ClamAV is not installed. Install with: sudo apt install clamav clamav-daemon${NC}" | tee -a "$LOG_FILE"
    exit 1
fi

# Step 2: Handle freshclam service
log_message "Checking freshclam service status..."
FRESHCLAM_RUNNING=false

if systemctl is-active --quiet clamav-freshclam; then
    FRESHCLAM_RUNNING=true
    log_message "Stopping freshclam service to avoid conflicts..."
    sudo systemctl stop clamav-freshclam
    sleep 2
fi

# Step 3: Update virus definitions
log_message "Updating virus definitions..."
echo "Updating virus database..."  # Simple output for TUI
if ! freshclam; then
    echo -e "${YELLOW}Warning: Failed to update virus definitions, continuing with existing database${NC}" | tee -a "$LOG_FILE"
else
    log_message "Virus definitions updated successfully"
fi

# Step 4: Restart freshclam service if it was running
if [ "$FRESHCLAM_RUNNING" = true ]; then
    log_message "Restarting freshclam service..."
    sudo systemctl start clamav-freshclam
fi

# Step 5: Define excluded directories
EXCLUDE_DIRS="--exclude-dir=/proc --exclude-dir=/sys --exclude-dir=/dev --exclude-dir=/snap --exclude-dir=/run --exclude-dir=/tmp"

# Step 6: Count total files (IMPORTANT: This is what the TUI looks for)
log_message "Counting files on system..."
echo "Counting files on system (for progress bar)..."

# Count files - this output MUST match what the TUI is looking for
FILE_COUNT=$(find / -type f \
    ! -path "/proc/*" \
    ! -path "/sys/*" \
    ! -path "/dev/*" \
    ! -path "/snap/*" \
    ! -path "/run/*" \
    ! -path "/tmp/*" \
    2>/dev/null | wc -l)

# CRITICAL: This exact line format is what the TUI parses for file count
echo "Total files to scan: $FILE_COUNT"

log_message "Found $FILE_COUNT files to scan"

# Step 7: Start the full system scan
log_message "Starting full system scan..."
echo "Starting ClamAV scan..."
echo -e "${YELLOW}This may take several hours depending on your system size...${NC}"
echo -e "${YELLOW}Do not interrupt the scan!${NC}"

# Initialize scanned count
SCANNED_COUNT=0

# Run scan and output to both report and stdout
# Using unbuffered output so TUI gets it in real-time
stdbuf -o0 clamscan --verbose \
    --recursive \
    --infected \
    --remove \
    --detect-pua \
    --scan-archive=yes \
    --max-filesize=512M \
    --max-scansize=512M \
    $EXCLUDE_DIRS \
    / 2>&1 | tee "$SCAN_REPORT" | while IFS= read -r line; do
    
    # Output the line as-is (TUI will read it)
    echo "$line"
    
    # Count scanned files when we see "Scanning" lines
    if [[ "$line" == *"Scanning"* ]]; then
        SCANNED_COUNT=$((SCANNED_COUNT + 1))
    fi
    
    # Flush output immediately
    if [ $((SCANNED_COUNT % 50)) -eq 0 ]; then
        sleep 0.01  # Small delay to prevent overwhelming the TUI
    fi
done

# Get exit status
SCAN_EXIT=${PIPESTATUS[0]}

# Step 8: Process results
echo ""
echo "==========================================" | tee -a "$LOG_FILE"
log_message "Scan completed with exit code: $SCAN_EXIT"

# Interpret results
case $SCAN_EXIT in
    0)
        echo "No viruses found!" | tee -a "$LOG_FILE"
        ;;
    1)
        echo "Virus(es) found and removed!" | tee -a "$LOG_FILE"
        ;;
    2)
        echo "Some files could not be scanned (check logs)" | tee -a "$LOG_FILE"
        ;;
    *)
        echo "Scan failed with unknown error (code: $SCAN_EXIT)" | tee -a "$LOG_FILE"
        ;;
esac

# Show summary
log_message "Scan report saved to: $SCAN_REPORT"
log_message "Full log saved to: $LOG_FILE"

# Display quick summary from report
if [ -f "$SCAN_REPORT" ]; then
    echo "------------------------------------------" | tee -a "$LOG_FILE"
    echo "SCAN SUMMARY:" | tee -a "$LOG_FILE"
    grep -E "(Infected files|Scanned files|Data scanned|Time:|Known viruses|Engine version)" "$SCAN_REPORT" | head -10 | tee -a "$LOG_FILE"
fi

log_message "Full system scan completed!"
echo "==========================================" | tee -a "$LOG_FILE"

# Final exit code for TUI
exit $SCAN_EXIT
