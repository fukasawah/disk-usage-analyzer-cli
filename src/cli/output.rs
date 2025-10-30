//! Output formatting for CLI

use crate::models::DirectoryEntry;
use crate::services::format::format_size;
use crate::Summary;

/// Strategy trait for determining whether to preview (expand) a directory's children
pub trait PreviewStrategy {
    /// Determine if this entry should be expanded to show its children
    /// 
    /// # Arguments
    /// * `entry` - The entry to evaluate
    /// * `parent_size` - The size of the parent directory
    /// * `root_size` - The size of the root directory (for global percentage)
    /// * `rank` - The rank of this entry among siblings (1-indexed)
    /// * `depth` - The depth in the tree (root = 0)
    fn should_preview(&self, entry: &DirectoryEntry, parent_size: u64, root_size: u64, rank: usize, depth: u16) -> bool;
    
    /// Maximum depth to preview
    fn max_preview_depth(&self) -> u16 {
        3
    }
    
    /// Maximum number of children to show per parent
    fn max_children_to_show(&self) -> usize {
        3
    }
}

/// Adaptive strategy: adjusts thresholds based on depth
pub struct AdaptivePreviewStrategy {
    pub depth_thresholds: Vec<(u16, f64)>, // (depth, ratio_threshold)
    pub min_rank_with_ratio: (usize, f64), // (rank, min_ratio)
    pub absolute_size_gb: u64,
}

impl Default for AdaptivePreviewStrategy {
    fn default() -> Self {
        Self {
            depth_thresholds: vec![
                (1, 0.30),  // 30% at depth 1
                (2, 0.40),  // 40% at depth 2
                (3, 0.50),  // 50% at depth 3
            ],
            min_rank_with_ratio: (3, 0.20), // Top 3 if >= 20%
            absolute_size_gb: 10,           // Always show if >= 10GB
        }
    }
}

impl PreviewStrategy for AdaptivePreviewStrategy {
    fn should_preview(&self, entry: &DirectoryEntry, parent_size: u64, _root_size: u64, rank: usize, depth: u16) -> bool {
        if parent_size == 0 {
            return false;
        }
        
        let parent_ratio = (entry.size_bytes as f64) / (parent_size as f64);
        
        // Check depth-based threshold
        for (d, threshold) in &self.depth_thresholds {
            if depth < *d {
                if parent_ratio >= *threshold {
                    return true;
                }
            }
        }
        
        // Fallback for deeper levels
        if depth >= self.depth_thresholds.last().map(|(d, _)| *d).unwrap_or(3) {
            if parent_ratio >= 0.60 {
                return true;
            }
        }
        
        // Check rank + ratio condition
        if rank <= self.min_rank_with_ratio.0 && parent_ratio >= self.min_rank_with_ratio.1 {
            return true;
        }
        
        // Check absolute size
        let size_gb = entry.size_bytes / (1024 * 1024 * 1024);
        if size_gb >= self.absolute_size_gb {
            return true;
        }
        
        false
    }
}

/// Simple strategy: always preview top N entries
pub struct SimplePreviewStrategy {
    pub top_n: usize,
}

impl PreviewStrategy for SimplePreviewStrategy {
    fn should_preview(&self, _entry: &DirectoryEntry, _parent_size: u64, _root_size: u64, rank: usize, _depth: u16) -> bool {
        rank <= self.top_n
    }
}

/// Format summary as human-readable text with hierarchical preview
pub fn format_text(summary: &Summary, entries: &[DirectoryEntry]) {
    format_text_with_all_entries(summary, entries, &[], &AdaptivePreviewStrategy::default())
}

/// Format summary with all entries available for preview
pub fn format_text_with_all_entries(
    summary: &Summary,
    entries: &[DirectoryEntry],
    all_entries: &[DirectoryEntry],
    strategy: &dyn PreviewStrategy,
) {
    if entries.is_empty() {
        println!("No entries found.");
        return;
    }
    
    // Calculate total size for root
    let root_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
    
    println!("{} ({})", summary.root, format_size(root_size));
    println!();
    println!("{:<70} {:>10} {:>5}", "Path", "Size", "%");
    println!("{}", "─".repeat(88));
    
    // Print entries with hierarchical preview
    print_entries_recursive(entries, all_entries, strategy, root_size, root_size, 0, 0);
    
    // Print errors if any
    if !summary.errors.is_empty() {
        println!();
        println!("Errors encountered: {}", summary.errors.len());
        if summary.errors.len() <= 5 {
            for error in &summary.errors {
                eprintln!("  {}: {}", error.path, error.message);
            }
        } else {
            for error in &summary.errors[..5] {
                eprintln!("  {}: {}", error.path, error.message);
            }
            eprintln!("  ... and {} more", summary.errors.len() - 5);
        }
    }
}

/// Format summary with a custom preview strategy (deprecated: use format_text_with_all_entries)
pub fn format_text_with_strategy(summary: &Summary, entries: &[DirectoryEntry], strategy: &dyn PreviewStrategy) {
    if entries.is_empty() {
        println!("No entries found.");
        return;
    }
    
    // Calculate total size for root
    let root_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
    
    println!("{} ({})", summary.root, format_size(root_size));
    println!();
    
    // Print entries with hierarchical preview
    print_entries_recursive(entries, &[], strategy, root_size, root_size, 0, 0);
    
    // Print errors if any
    if !summary.errors.is_empty() {
        println!();
        println!("Errors encountered: {}", summary.errors.len());
        if summary.errors.len() <= 5 {
            for error in &summary.errors {
                eprintln!("  {}: {}", error.path, error.message);
            }
        } else {
            for error in &summary.errors[..5] {
                eprintln!("  {}: {}", error.path, error.message);
            }
            eprintln!("  ... and {} more", summary.errors.len() - 5);
        }
    }
}

/// Get ANSI color code based on percentage
fn get_color_for_percentage(pct: f64) -> &'static str {
    if pct >= 30.0 {
        "\x1b[31m"  // Red for >= 30%
    } else if pct >= 15.0 {
        "\x1b[33m"  // Yellow for >= 15%
    } else if pct >= 5.0 {
        "\x1b[36m"  // Cyan for >= 5%
    } else {
        "\x1b[90m"  // Gray for < 5%
    }
}

/// Reset ANSI color
const COLOR_RESET: &str = "\x1b[0m";

/// Recursively print entries with preview
fn print_entries_recursive(
    current_entries: &[DirectoryEntry],
    all_entries: &[DirectoryEntry],
    strategy: &dyn PreviewStrategy,
    parent_size: u64,
    root_size: u64,
    indent_level: usize,
    current_depth: u16,
) {
    for (rank, entry) in current_entries.iter().enumerate() {
        let rank_1indexed = rank + 1;
        
        // Calculate percentage
        let pct = if root_size > 0 {
            (entry.size_bytes as f64 / root_size as f64) * 100.0
        } else {
            0.0
        };
        
        // Use full path
        let path = &entry.path;
        
        // Determine if this is a directory
        let is_dir = entry.dir_count > 0 || entry.file_count > 0;
        let path_display = if is_dir && !path.ends_with('/') {
            format!("{}/", path)
        } else {
            path.to_string()
        };
        
        // Color based on percentage
        let color = get_color_for_percentage(pct);
        
        println!(
            "{}{:<70}{} {:>10} {:>5.1}%",
            color,
            path_display,
            COLOR_RESET,
            format_size(entry.size_bytes),
            pct
        );
        
        // Determine if we should preview this entry's children
        if current_depth < strategy.max_preview_depth()
            && strategy.should_preview(entry, parent_size, root_size, rank_1indexed, current_depth)
        {
            // Get children of this entry
            let children = if all_entries.is_empty() {
                // First call: use current_entries as the data source
                vec![]  // Will be handled by parent
            } else {
                get_children_from_all(all_entries, &entry.path, entry.depth)
            };
            
            if !children.is_empty() {
                let max_children = strategy.max_children_to_show().min(children.len());
                let children_to_show = &children[..max_children];
                
                print_entries_recursive(
                    children_to_show,
                    all_entries,
                    strategy,
                    entry.size_bytes,
                    root_size,
                    indent_level + 1,
                    current_depth + 1,
                );
            }
        }
    }
}

/// Get immediate children of a directory from all entries
fn get_children_from_all(all_entries: &[DirectoryEntry], parent_path: &str, parent_depth: u16) -> Vec<DirectoryEntry> {
    use crate::services::aggregate::get_immediate_children;
    let mut children = get_immediate_children(all_entries, parent_path, parent_depth);
    children.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    children
}

/// Format summary as JSON
pub fn format_json(summary: &Summary, entries: &[DirectoryEntry]) -> String {
    let output = serde_json::json!({
        "root": summary.root,
        "entries": entries,
        "error_count": summary.errors.len(),
        "errors": if summary.errors.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::json!(summary.errors)
        }
    });
    
    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
}
