use serde::Serialize;

use super::Cache;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(super) struct Point {
    pub entries: usize,
    pub capacity: usize,
    pub payload_bytes: usize,
    pub visible_payload_bytes: usize,
}

impl Point {
    pub fn observe(cache: &Cache) -> Self {
        Self {
            entries: cache.lines.len(),
            capacity: cache.lines.capacity(),
            payload_bytes: cache.bytes,
            visible_payload_bytes: cache
                .visible
                .iter()
                .filter_map(|id| cache.lines.get(id))
                .map(super::LineStyle::cache_weight)
                .sum(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
struct Admission {
    ordinal: usize,
    before: Point,
    inserted: Point,
    settled: Point,
    prior_peak_entries: usize,
    prior_capacity_high_water: usize,
}

#[derive(Default, Debug, Serialize)]
struct Increases {
    count: usize,
    at_capacity: usize,
    with_eviction: usize,
    without_new_entry_peak: usize,
    within_prior_capacity_high_water: usize,
    last: Option<Admission>,
}

impl Increases {
    fn record(&mut self, event: Admission) {
        self.count += 1;
        self.at_capacity += usize::from(event.before.entries == event.before.capacity);
        self.with_eviction += usize::from(event.inserted.entries > event.settled.entries);
        self.without_new_entry_peak +=
            usize::from(event.inserted.entries <= event.prior_peak_entries);
        self.within_prior_capacity_high_water +=
            usize::from(event.inserted.entries <= event.prior_capacity_high_water);
        self.last = Some(event);
    }
}

#[derive(Default, Debug, Serialize)]
pub(super) struct Activity {
    admissions: usize,
    pub duplicates: usize,
    evictions: usize,
    admissions_with_eviction: usize,
    admitted_payload_bytes: usize,
    evicted_payload_bytes: usize,
    invalidations: usize,
    invalidated_entries: usize,
    invalidated_payload_bytes: usize,
    invalidation_capacity_gained: usize,
    invalidation_capacity_lost: usize,
    releases: usize,
    released_entries: usize,
    released_payload_bytes: usize,
    release_capacity_gained: usize,
    release_capacity_lost: usize,
    peak_entries: usize,
    peak_settled_entries: usize,
    peak_payload_bytes: usize,
    peak_settled_payload_bytes: usize,
    peak_visible_payload_bytes: usize,
    capacity_high_water: usize,
    insertion_capacity_gained: usize,
    insertion_capacity_lost: usize,
    removal_capacity_gained: usize,
    removal_capacity_lost: usize,
    capacity_increases: Increases,
    high_water_increases: Increases,
}

impl Activity {
    pub fn admit(&mut self, before: Point, inserted: Point, settled: Point) {
        self.admissions += 1;
        self.admitted_payload_bytes += inserted.payload_bytes - before.payload_bytes;
        self.evicted_payload_bytes += inserted.payload_bytes - settled.payload_bytes;
        let event = Admission {
            ordinal: self.admissions,
            before,
            inserted,
            settled,
            prior_peak_entries: self.peak_entries,
            prior_capacity_high_water: self.capacity_high_water,
        };
        if inserted.capacity > before.capacity {
            self.capacity_increases.record(event);
        }
        if inserted.capacity > self.capacity_high_water {
            self.high_water_increases.record(event);
        }
        self.evictions += inserted.entries - settled.entries;
        self.admissions_with_eviction += usize::from(inserted.entries > settled.entries);
        self.peak_entries = self.peak_entries.max(inserted.entries);
        self.peak_settled_entries = self.peak_settled_entries.max(settled.entries);
        self.peak_payload_bytes = self.peak_payload_bytes.max(inserted.payload_bytes);
        self.peak_settled_payload_bytes =
            self.peak_settled_payload_bytes.max(settled.payload_bytes);
        self.peak_visible_payload_bytes = self
            .peak_visible_payload_bytes
            .max(settled.visible_payload_bytes);
        self.capacity_high_water = self
            .capacity_high_water
            .max(inserted.capacity)
            .max(settled.capacity);
        self.insertion_capacity_gained += inserted.capacity.saturating_sub(before.capacity);
        self.insertion_capacity_lost += before.capacity.saturating_sub(inserted.capacity);
        self.removal_capacity_gained += settled.capacity.saturating_sub(inserted.capacity);
        self.removal_capacity_lost += inserted.capacity.saturating_sub(settled.capacity);
    }

    pub fn invalidate(&mut self, before: Point, after: Point) {
        self.invalidations += 1;
        self.invalidated_entries += before.entries - after.entries;
        self.invalidated_payload_bytes += before.payload_bytes - after.payload_bytes;
        self.invalidation_capacity_gained += after.capacity.saturating_sub(before.capacity);
        self.invalidation_capacity_lost += before.capacity.saturating_sub(after.capacity);
    }

    pub fn release(&mut self, before: Point, after: Point) {
        self.releases += 1;
        self.released_entries += before.entries - after.entries;
        self.released_payload_bytes += before.payload_bytes - after.payload_bytes;
        self.release_capacity_gained += after.capacity.saturating_sub(before.capacity);
        self.release_capacity_lost += before.capacity.saturating_sub(after.capacity);
        self.peak_visible_payload_bytes = self
            .peak_visible_payload_bytes
            .max(after.visible_payload_bytes);
    }
}

#[cfg(test)]
#[path = "cache_activity/source_tests.rs"]
mod source_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlight::LineStyle;
    use std::ops::Range;

    #[test]
    fn real_admissions_reconcile_peaks_duplicates_evictions_and_capacity() {
        let mut cache = Cache::default();
        let mut settled = Vec::new();
        let mut peak_entries = 0;
        let mut peak_payload = 0;
        let mut last_high_water = 0;
        let mut high_water_events = 0;
        for (id, mib) in [(1, 6), (2, 6), (3, 6), (4, 6), (5, 17), (6, 33), (1, 1)] {
            let style = LineStyle {
                changed: Vec::with_capacity(mib * 1024 * 1024 / size_of::<Range<usize>>()),
                ..LineStyle::default()
            };
            let before = Point::observe(&cache);
            peak_entries = peak_entries.max(before.entries + 1);
            peak_payload = peak_payload.max(before.payload_bytes + style.cache_weight());
            cache.insert(id, style);
            let after = Point::observe(&cache);
            if after.capacity > last_high_water {
                high_water_events += 1;
                let event = cache.activity.high_water_increases.last.unwrap();
                assert_eq!(event.before, before);
                assert_eq!(event.inserted.entries, before.entries + 1);
                assert_eq!(event.settled, after);
                assert_eq!(event.prior_capacity_high_water, last_high_water);
                assert_eq!(event.ordinal, settled.len() + 1);
                last_high_water = after.capacity;
            }
            settled.push(after);
            let a = &cache.activity;
            assert_eq!(a.admissions, settled.len());
            assert_eq!(a.admissions - a.evictions, after.entries);
            assert_eq!(
                a.admitted_payload_bytes - a.evicted_payload_bytes,
                after.payload_bytes
            );
            assert_eq!(a.peak_entries, peak_entries);
            assert_eq!(a.peak_payload_bytes, peak_payload);
            assert_eq!(
                a.peak_settled_entries,
                settled.iter().map(|p| p.entries).max().unwrap()
            );
            assert_eq!(
                a.peak_settled_payload_bytes,
                settled.iter().map(|p| p.payload_bytes).max().unwrap()
            );
            assert_eq!(a.high_water_increases.count, high_water_events);
            assert_eq!(
                a.insertion_capacity_gained + a.removal_capacity_gained,
                after.capacity + a.insertion_capacity_lost + a.removal_capacity_lost
            );
        }
        assert_eq!(cache.activity.evictions, 6);
        assert_eq!(cache.activity.admissions_with_eviction, 2);
        let mut expected = serde_json::to_value(&cache.activity).unwrap();
        expected["duplicates"] = 1.into();
        cache.insert(1, LineStyle::default());
        assert_eq!(serde_json::to_value(&cache.activity).unwrap(), expected);
        assert_eq!(Point::observe(&cache), *settled.last().unwrap());
        let census = cache.census();
        assert_eq!(cache.census(), census);
        assert_eq!(census["activity"]["observations"], expected);
        assert_eq!(Cache::default().activity.admissions, 0);
    }

    #[test]
    fn real_table_churn_reconciles_capacity_classifications() {
        use hashbrown::HashMap;
        use std::collections::VecDeque;

        let mut cache = Cache::default();
        let mut table = HashMap::with_hasher(cache.lines.hasher().clone());
        let mut queue = VecDeque::new();
        let mut payload = 0;
        let mut peak_entries = 0;
        let mut capacity_high_water = 0;
        let mut expected = [[0; 5]; 2];
        let mut removal_loss = 0;
        for id in 0..16_384 {
            let style = LineStyle {
                changed: Vec::with_capacity(32 * 1024 / size_of::<Range<usize>>()),
                ..LineStyle::default()
            };
            let weight = style.cache_weight();
            let before = Point::observe(&cache);
            table.insert(crate::view::LineId::Patch(id), ());
            queue.push_back((crate::view::LineId::Patch(id), weight));
            payload += weight;
            let inserted = Point {
                entries: table.len(),
                capacity: table.capacity(),
                payload_bytes: payload,
                visible_payload_bytes: 0,
            };
            while payload > 32 * 1024 * 1024 {
                let (old, weight) = queue.pop_front().unwrap();
                assert!(table.remove(&old).is_some());
                payload -= weight;
            }
            let settled = Point {
                entries: table.len(),
                capacity: table.capacity(),
                payload_bytes: payload,
                visible_payload_bytes: 0,
            };
            cache.insert(id, style);
            assert_eq!(Point::observe(&cache), settled);
            removal_loss += inserted.capacity - settled.capacity;
            for (index, increased) in [
                inserted.capacity > before.capacity,
                inserted.capacity > capacity_high_water,
            ]
            .into_iter()
            .enumerate()
            {
                if increased {
                    for (slot, condition) in [
                        true,
                        before.entries == before.capacity,
                        inserted.entries > settled.entries,
                        inserted.entries <= peak_entries,
                        inserted.entries <= capacity_high_water,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        expected[index][slot] += usize::from(condition);
                    }
                    let group = if index == 0 {
                        &cache.activity.capacity_increases
                    } else {
                        &cache.activity.high_water_increases
                    };
                    let event = group.last.unwrap();
                    assert_eq!(event.ordinal, id + 1);
                    assert_eq!(event.before, before);
                    assert_eq!(event.inserted, inserted);
                    assert_eq!(event.settled, settled);
                    assert_eq!(event.prior_peak_entries, peak_entries);
                    assert_eq!(event.prior_capacity_high_water, capacity_high_water);
                }
            }
            for (group, expected) in [
                &cache.activity.capacity_increases,
                &cache.activity.high_water_increases,
            ]
            .into_iter()
            .zip(expected)
            {
                assert_eq!(
                    [
                        group.count,
                        group.at_capacity,
                        group.with_eviction,
                        group.without_new_entry_peak,
                        group.within_prior_capacity_high_water
                    ],
                    expected
                );
            }
            peak_entries = peak_entries.max(inserted.entries);
            capacity_high_water = capacity_high_water.max(inserted.capacity);
            assert_eq!(cache.activity.peak_entries, peak_entries);
            assert_eq!(cache.activity.capacity_high_water, capacity_high_water);
            assert_eq!(cache.activity.removal_capacity_lost, removal_loss);
            assert_eq!(
                cache.activity.admissions - cache.activity.evictions,
                table.len()
            );
        }
        assert!(cache.activity.evictions > cache.lines.len());
        assert!(removal_loss > 0);
        assert!(expected[0][0] > expected[1][0]);
        assert!(expected[1][4] > 0);
    }
}
