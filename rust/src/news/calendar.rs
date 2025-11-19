//! Economic Calendar Module
//!
//! Tracks high-impact economic events that affect Gold/USD trading.
//! Helps avoid trading during volatile news releases.

use chrono::{DateTime, Datelike, Duration, NaiveTime, TimeZone, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Impact level of economic event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventImpact {
    /// Low impact - minor price movement expected
    Low,
    /// Medium impact - moderate volatility
    Medium,
    /// High impact - significant volatility expected
    High,
}

/// Economic event type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// Federal Reserve FOMC meeting
    FOMC,
    /// Non-Farm Payrolls
    NFP,
    /// Consumer Price Index
    CPI,
    /// Producer Price Index
    PPI,
    /// GDP Release
    GDP,
    /// Unemployment Claims
    UnemploymentClaims,
    /// Personal Consumption Expenditures
    PCE,
    /// Retail Sales
    RetailSales,
    /// Fed Chair Speech
    FedSpeech,
    /// Other high-impact event
    Other(String),
}

impl EventType {
    pub fn name(&self) -> &str {
        match self {
            EventType::FOMC => "FOMC Meeting",
            EventType::NFP => "Non-Farm Payrolls",
            EventType::CPI => "CPI Release",
            EventType::PPI => "PPI Release",
            EventType::GDP => "GDP Release",
            EventType::UnemploymentClaims => "Unemployment Claims",
            EventType::PCE => "PCE Release",
            EventType::RetailSales => "Retail Sales",
            EventType::FedSpeech => "Fed Chair Speech",
            EventType::Other(name) => name,
        }
    }

    pub fn default_impact(&self) -> EventImpact {
        match self {
            EventType::FOMC => EventImpact::High,
            EventType::NFP => EventImpact::High,
            EventType::CPI => EventImpact::High,
            EventType::PPI => EventImpact::Medium,
            EventType::GDP => EventImpact::High,
            EventType::UnemploymentClaims => EventImpact::Medium,
            EventType::PCE => EventImpact::High,
            EventType::RetailSales => EventImpact::Medium,
            EventType::FedSpeech => EventImpact::High,
            EventType::Other(_) => EventImpact::Medium,
        }
    }
}

/// Economic event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicEvent {
    pub event_type: EventType,
    pub scheduled_time: DateTime<Utc>,
    pub impact: EventImpact,
    pub description: String,
}

impl EconomicEvent {
    pub fn new(
        event_type: EventType,
        scheduled_time: DateTime<Utc>,
        impact: EventImpact,
        description: String,
    ) -> Self {
        Self {
            event_type,
            scheduled_time,
            impact,
            description,
        }
    }

    /// Check if this event is upcoming within the given window
    pub fn is_upcoming(&self, current_time: DateTime<Utc>, window_minutes: i64) -> bool {
        let time_until_event = self.scheduled_time.signed_duration_since(current_time);
        time_until_event >= Duration::zero()
            && time_until_event <= Duration::minutes(window_minutes)
    }

    /// Check if we're currently in the event window
    pub fn is_active(&self, current_time: DateTime<Utc>, window_minutes: i64) -> bool {
        let time_diff = current_time.signed_duration_since(self.scheduled_time);
        time_diff >= Duration::minutes(-window_minutes)
            && time_diff <= Duration::minutes(window_minutes)
    }
}

/// Economic calendar configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarConfig {
    /// Minutes before event to close positions
    pub pre_event_close_minutes: i64,
    /// Minutes before event to stop opening new positions
    pub pre_event_restriction_minutes: i64,
    /// Minutes after event to resume trading
    pub post_event_resume_minutes: i64,
    /// Minimum impact level to trigger restrictions
    pub min_impact_level: EventImpact,
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            pre_event_close_minutes: 30,
            pre_event_restriction_minutes: 60,
            post_event_resume_minutes: 30,
            min_impact_level: EventImpact::High,
        }
    }
}

/// Economic calendar manager
pub struct EconomicCalendar {
    config: CalendarConfig,
    events: Vec<EconomicEvent>,
}

impl EconomicCalendar {
    pub fn new(config: CalendarConfig) -> Self {
        Self {
            config,
            events: Vec::new(),
        }
    }

    /// Add an event to the calendar
    pub fn add_event(&mut self, event: EconomicEvent) {
        self.events.push(event);
        // Keep events sorted by time
        self.events
            .sort_by(|a, b| a.scheduled_time.cmp(&b.scheduled_time));
    }

    /// Load typical recurring events for the current month
    pub fn load_typical_events(&mut self, year: i32, month: u32) {
        // NFP: First Friday of each month at 8:30 AM ET (13:30 UTC)
        if let Some(nfp_date) = Self::find_nth_weekday_of_month(year, month, Weekday::Fri, 1) {
            let nfp_time = Utc
                .with_ymd_and_hms(
                    nfp_date.year(),
                    nfp_date.month(),
                    nfp_date.day(),
                    13,
                    30,
                    0,
                )
                .unwrap();
            self.add_event(EconomicEvent::new(
                EventType::NFP,
                nfp_time,
                EventImpact::High,
                "Non-Farm Payrolls Report".to_string(),
            ));
        }

        // CPI: Mid-month (typically 13th) at 8:30 AM ET (13:30 UTC)
        let cpi_day = 13;
        if Self::is_valid_date(year, month, cpi_day) {
            let cpi_time = Utc.with_ymd_and_hms(year, month, cpi_day, 13, 30, 0).unwrap();
            self.add_event(EconomicEvent::new(
                EventType::CPI,
                cpi_time,
                EventImpact::High,
                "Consumer Price Index".to_string(),
            ));
        }

        // FOMC meetings (8 per year, roughly every 6 weeks)
        // Simplified: add on 3rd Wednesday of Jan, Mar, May, Jun, Jul, Sep, Nov, Dec
        let fomc_months = [1, 3, 5, 6, 7, 9, 11, 12];
        if fomc_months.contains(&month) {
            if let Some(fomc_date) =
                Self::find_nth_weekday_of_month(year, month, Weekday::Wed, 3)
            {
                let fomc_time = Utc
                    .with_ymd_and_hms(
                        fomc_date.year(),
                        fomc_date.month(),
                        fomc_date.day(),
                        19,
                        0,
                        0,
                    )
                    .unwrap();
                self.add_event(EconomicEvent::new(
                    EventType::FOMC,
                    fomc_time,
                    EventImpact::High,
                    "FOMC Meeting Announcement".to_string(),
                ));
            }
        }
    }

    /// Check if trading should be restricted at current time
    pub fn should_restrict_trading(&self, current_time: DateTime<Utc>) -> Option<&EconomicEvent> {
        for event in &self.events {
            if event.impact as i32 >= self.config.min_impact_level as i32 {
                let time_until_event = event.scheduled_time.signed_duration_since(current_time);
                let time_since_event = current_time.signed_duration_since(event.scheduled_time);

                // Before event: check restriction window
                if time_until_event >= Duration::zero()
                    && time_until_event
                        <= Duration::minutes(self.config.pre_event_restriction_minutes)
                {
                    return Some(event);
                }

                // After event: check resume window
                if time_since_event >= Duration::zero()
                    && time_since_event <= Duration::minutes(self.config.post_event_resume_minutes)
                {
                    return Some(event);
                }
            }
        }
        None
    }

    /// Check if positions should be closed due to upcoming event
    pub fn should_close_positions(&self, current_time: DateTime<Utc>) -> Option<&EconomicEvent> {
        for event in &self.events {
            if event.impact as i32 >= self.config.min_impact_level as i32 {
                let time_until_event = event.scheduled_time.signed_duration_since(current_time);

                if time_until_event >= Duration::zero()
                    && time_until_event <= Duration::minutes(self.config.pre_event_close_minutes)
                {
                    return Some(event);
                }
            }
        }
        None
    }

    /// Get upcoming events within the next N hours
    pub fn get_upcoming_events(&self, current_time: DateTime<Utc>, hours: i64) -> Vec<&EconomicEvent> {
        self.events
            .iter()
            .filter(|event| {
                let time_until = event.scheduled_time.signed_duration_since(current_time);
                time_until >= Duration::zero() && time_until <= Duration::hours(hours)
            })
            .collect()
    }

    /// Clean up past events
    pub fn clean_old_events(&mut self, current_time: DateTime<Utc>) {
        self.events.retain(|event| {
            let time_since = current_time.signed_duration_since(event.scheduled_time);
            time_since < Duration::hours(24)
        });
    }

    /// Helper: Find the nth occurrence of a weekday in a month
    fn find_nth_weekday_of_month(
        year: i32,
        month: u32,
        weekday: Weekday,
        nth: u32,
    ) -> Option<DateTime<Utc>> {
        let mut count = 0;
        for day in 1..=31 {
            if !Self::is_valid_date(year, month, day) {
                break;
            }

            if let Some(date) = Utc.with_ymd_and_hms(year, month, day, 0, 0, 0).single() {
                if date.weekday() == weekday {
                    count += 1;
                    if count == nth {
                        return Some(date);
                    }
                }
            }
        }
        None
    }

    /// Helper: Check if date is valid
    fn is_valid_date(year: i32, month: u32, day: u32) -> bool {
        Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
            .single()
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_is_upcoming() {
        let event = EconomicEvent::new(
            EventType::NFP,
            Utc.with_ymd_and_hms(2025, 1, 10, 13, 30, 0).unwrap(),
            EventImpact::High,
            "NFP".to_string(),
        );

        // 20 minutes before event
        let current = Utc.with_ymd_and_hms(2025, 1, 10, 13, 10, 0).unwrap();
        assert!(event.is_upcoming(current, 30));

        // 40 minutes before event
        let current = Utc.with_ymd_and_hms(2025, 1, 10, 12, 50, 0).unwrap();
        assert!(!event.is_upcoming(current, 30));
    }

    #[test]
    fn test_should_restrict_trading() {
        let config = CalendarConfig {
            pre_event_close_minutes: 30,
            pre_event_restriction_minutes: 60,
            post_event_resume_minutes: 30,
            min_impact_level: EventImpact::High,
        };

        let mut calendar = EconomicCalendar::new(config);

        let event_time = Utc.with_ymd_and_hms(2025, 1, 10, 13, 30, 0).unwrap();
        calendar.add_event(EconomicEvent::new(
            EventType::NFP,
            event_time,
            EventImpact::High,
            "NFP".to_string(),
        ));

        // 45 minutes before - should restrict
        let current = Utc.with_ymd_and_hms(2025, 1, 10, 12, 45, 0).unwrap();
        assert!(calendar.should_restrict_trading(current).is_some());

        // 90 minutes before - should not restrict
        let current = Utc.with_ymd_and_hms(2025, 1, 10, 12, 0, 0).unwrap();
        assert!(calendar.should_restrict_trading(current).is_none());

        // 20 minutes after - should restrict
        let current = Utc.with_ymd_and_hms(2025, 1, 10, 13, 50, 0).unwrap();
        assert!(calendar.should_restrict_trading(current).is_some());
    }

    #[test]
    fn test_load_typical_events() {
        let config = CalendarConfig::default();
        let mut calendar = EconomicCalendar::new(config);

        calendar.load_typical_events(2025, 1);

        assert!(!calendar.events.is_empty());

        // Should have NFP for first Friday
        let nfp_events: Vec<_> = calendar
            .events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::NFP))
            .collect();
        assert_eq!(nfp_events.len(), 1);
    }
}
