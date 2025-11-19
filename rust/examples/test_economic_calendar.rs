//! Demonstration of Economic Calendar Integration

use chrono::{Duration, TimeZone, Utc};
use gold_quant_trading::news::{CalendarConfig, EconomicCalendar, EconomicEvent, EventImpact, EventType};

fn main() {
    println!("\n=== Economic Calendar Integration Demonstration ===\n");

    // Create calendar with default config
    let config = CalendarConfig {
        pre_event_close_minutes: 30,
        pre_event_restriction_minutes: 60,
        post_event_resume_minutes: 30,
        min_impact_level: EventImpact::High,
    };

    println!("Calendar Configuration:");
    println!("  Close positions {} minutes before high-impact events", config.pre_event_close_minutes);
    println!("  Restrict new trades {} minutes before events", config.pre_event_restriction_minutes);
    println!("  Resume trading {} minutes after events", config.post_event_resume_minutes);
    println!("  Minimum impact level: {:?}\n", config.min_impact_level);

    let mut calendar = EconomicCalendar::new(config);

    // Add some example events
    let now = Utc::now();

    // NFP in 2 hours
    let nfp_time = now + Duration::hours(2);
    calendar.add_event(EconomicEvent::new(
        EventType::NFP,
        nfp_time,
        EventImpact::High,
        "Non-Farm Payrolls Report".to_string(),
    ));

    // FOMC tomorrow
    let fomc_time = now + Duration::hours(26);
    calendar.add_event(EconomicEvent::new(
        EventType::FOMC,
        fomc_time,
        EventImpact::High,
        "FOMC Meeting Announcement".to_string(),
    ));

    // CPI in 3 days
    let cpi_time = now + Duration::hours(72);
    calendar.add_event(EconomicEvent::new(
        EventType::CPI,
        cpi_time,
        EventImpact::High,
        "Consumer Price Index".to_string(),
    ));

    println!("Loaded {} economic events\n", calendar.get_upcoming_events(now, 720).len());

    // Simulate different times relative to events
    let scenarios = vec![
        (now, "Current time - No events imminent"),
        (nfp_time - Duration::minutes(45), "45 minutes before NFP - Should restrict trading"),
        (nfp_time - Duration::minutes(20), "20 minutes before NFP - Should close positions"),
        (nfp_time + Duration::minutes(15), "15 minutes after NFP - Still in restriction window"),
        (nfp_time + Duration::minutes(45), "45 minutes after NFP - Trading resumed"),
    ];

    for (test_time, description) in scenarios {
        println!("--- {} ---", description);

        // Check for upcoming events
        let upcoming = calendar.get_upcoming_events(test_time, 24);
        if !upcoming.is_empty() {
            println!("  Upcoming events in next 24h:");
            for event in &upcoming {
                let hours_until = event.scheduled_time.signed_duration_since(test_time).num_hours();
                println!("    - {} in {}h ({:?})", event.event_type.name(), hours_until, event.impact);
            }
        }

        // Check if trading should be restricted
        if let Some(event) = calendar.should_restrict_trading(test_time) {
            let time_diff = event.scheduled_time.signed_duration_since(test_time);
            if time_diff.num_minutes() > 0 {
                println!("  ⚠️  Trading RESTRICTED: {} in {} minutes",
                    event.event_type.name(), time_diff.num_minutes());
            } else {
                println!("  ⚠️  Trading RESTRICTED: {} ended {} minutes ago",
                    event.event_type.name(), -time_diff.num_minutes());
            }
        } else {
            println!("  ✅ Trading ALLOWED");
        }

        // Check if positions should be closed
        if let Some(event) = calendar.should_close_positions(test_time) {
            let mins_until = event.scheduled_time.signed_duration_since(test_time).num_minutes();
            println!("  🚨 CLOSE POSITIONS: {} in {} minutes!", event.event_type.name(), mins_until);
        }

        println!();
    }

    // Load typical events for current month
    println!("=== Loading Typical Monthly Events ===\n");
    calendar.load_typical_events(now.year(), now.month());

    let all_upcoming = calendar.get_upcoming_events(now, 720);
    println!("Total events loaded for this month: {}", all_upcoming.len());

    for event in all_upcoming.iter().take(5) {
        let days_until = event.scheduled_time.signed_duration_since(now).num_days();
        println!(
            "  - {} on {} (in {} days, {:?} impact)",
            event.event_type.name(),
            event.scheduled_time.format("%Y-%m-%d %H:%M UTC"),
            days_until,
            event.impact
        );
    }

    println!("\n=== Demonstration Complete ===");
    println!("\nKey Benefits:");
    println!("  ✅ Automatically closes positions 30 minutes before high-impact events");
    println!("  ✅ Prevents new trades 60 minutes before and 30 minutes after events");
    println!("  ✅ Avoids volatile whipsaws during FOMC, NFP, CPI releases");
    println!("  ✅ Loads typical recurring events automatically");
    println!("  ✅ Can be extended with real-time calendar API integration\n");
}
