// Recommended wiring code for the main task dispatcher
//
// Add this match arm inside your central task processing function
// (typically in task_management.rs or workers/mod.rs)

use crate::workers::wireless_active_handler::handle_wireless_active_task;

// Example inside the task dispatcher:
//
// match task_config {
//     TaskConfig::WirelessActive {
//         interface,
//         attack_type,
//         bssid,
//         client,
//         frame_count,
//         rate_limit,
//         dry_run,
//     } => {
//         let result = handle_wireless_active_task(
//             interface,
//             attack_type,
//             bssid,
//             client,
//             frame_count,
//             rate_limit,
//             dry_run,
//         ).await;
//
//         match result {
//             Ok(res) => {
//                 // Send result back to the WirelessTab
//                 // e.g. tab.set_active_results(res);
//             }
//             Err(e) => {
//                 // e.g. tab.set_error(TabError::new(e.to_string()));
//             }
//         }
//     }
//     // ... other task variants
// }

// Call this during initialization:
// crate::workers::wireless_active_handler::register_wireless_active_handler();
