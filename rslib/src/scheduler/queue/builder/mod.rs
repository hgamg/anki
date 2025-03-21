// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

mod gathering;
pub(crate) mod intersperser;
pub(crate) mod sized_chain;
mod sorting;

use std::collections::{HashMap, VecDeque};

use intersperser::Intersperser;
use sized_chain::SizedChain;

use super::{CardQueues, Counts, LearningQueueEntry, MainQueueEntry, MainQueueEntryKind};
use crate::{
    deckconfig::{NewCardGatherPriority, NewCardSortOrder, ReviewCardOrder, ReviewMix},
    decks::limits::{remaining_limits_map, RemainingLimits},
    prelude::*,
};

/// Temporary holder for review cards that will be built into a queue.
#[derive(Debug, Clone)]
pub(crate) struct DueCard {
    pub id: CardId,
    pub note_id: NoteId,
    pub mtime: TimestampSecs,
    pub due: i32,
    pub current_deck_id: DeckId,
    pub original_deck_id: DeckId,
    pub kind: DueCardKind,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DueCardKind {
    Review,
    Learning,
}

/// Temporary holder for new cards that will be built into a queue.
#[derive(Debug, Default, Clone)]
pub(crate) struct NewCard {
    pub id: CardId,
    pub note_id: NoteId,
    pub mtime: TimestampSecs,
    pub due: i32,
    pub original_deck_id: DeckId,
    pub template_index: u32,
    pub hash: u64,
}

impl From<DueCard> for MainQueueEntry {
    fn from(c: DueCard) -> Self {
        MainQueueEntry {
            id: c.id,
            mtime: c.mtime,
            kind: match c.kind {
                DueCardKind::Review => MainQueueEntryKind::Review,
                DueCardKind::Learning => MainQueueEntryKind::InterdayLearning,
            },
        }
    }
}

impl From<NewCard> for MainQueueEntry {
    fn from(c: NewCard) -> Self {
        MainQueueEntry {
            id: c.id,
            mtime: c.mtime,
            kind: MainQueueEntryKind::New,
        }
    }
}

impl From<DueCard> for LearningQueueEntry {
    fn from(c: DueCard) -> Self {
        LearningQueueEntry {
            due: TimestampSecs(c.due as i64),
            id: c.id,
            mtime: c.mtime,
        }
    }
}

/// When we encounter a card with new or review burying enabled, all future
/// siblings need to be buried, regardless of their own settings.
#[derive(Default, Debug, Clone, Copy)]
pub(super) struct BuryMode {
    bury_new: bool,
    bury_reviews: bool,
}

#[derive(Default, Clone, Debug)]
pub(super) struct QueueSortOptions {
    pub(super) new_order: NewCardSortOrder,
    pub(super) new_gather_priority: NewCardGatherPriority,
    pub(super) review_order: ReviewCardOrder,
    pub(super) day_learn_mix: ReviewMix,
    pub(super) new_review_mix: ReviewMix,
}

#[derive(Default)]
pub(super) struct QueueBuilder {
    pub(super) new: Vec<NewCard>,
    pub(super) review: Vec<DueCard>,
    pub(super) learning: Vec<DueCard>,
    pub(super) day_learning: Vec<DueCard>,
    pub(super) seen_note_ids: HashMap<NoteId, BuryMode>,
    pub(super) sort_options: QueueSortOptions,
}

impl QueueBuilder {
    pub(super) fn new(sort_options: QueueSortOptions) -> Self {
        QueueBuilder {
            sort_options,
            ..Default::default()
        }
    }

    pub(super) fn build(
        mut self,
        top_deck_limits: RemainingLimits,
        learn_ahead_secs: i64,
        current_day: u32,
    ) -> CardQueues {
        self.sort_new();

        // intraday learning and total learn count
        let intraday_learning = sort_learning(self.learning);
        let now = TimestampSecs::now();
        let cutoff = now.adding_secs(learn_ahead_secs);
        let learn_count = intraday_learning
            .iter()
            .take_while(|e| e.due <= cutoff)
            .count()
            + self.day_learning.len();

        // cap and note down review + new counts
        self.review.truncate(top_deck_limits.review as usize);
        let review_count = self.review.len();
        self.new.truncate(top_deck_limits.new as usize);
        let new_count = self.new.len();

        // merge interday and new cards into main
        let with_interday_learn = merge_day_learning(
            self.review,
            self.day_learning,
            self.sort_options.day_learn_mix,
        );
        let main_iter = merge_new(
            with_interday_learn,
            self.new,
            self.sort_options.new_review_mix,
        );

        CardQueues {
            counts: Counts {
                new: new_count,
                review: review_count,
                learning: learn_count,
            },
            main: main_iter.collect(),
            intraday_learning,
            learn_ahead_secs,
            current_day,
            build_time: TimestampMillis::now(),
            current_learning_cutoff: now,
        }
    }
}

fn merge_day_learning(
    reviews: Vec<DueCard>,
    day_learning: Vec<DueCard>,
    mode: ReviewMix,
) -> Box<dyn ExactSizeIterator<Item = MainQueueEntry>> {
    let day_learning_iter = day_learning.into_iter().map(Into::into);
    let reviews_iter = reviews.into_iter().map(Into::into);

    match mode {
        ReviewMix::AfterReviews => Box::new(SizedChain::new(reviews_iter, day_learning_iter)),
        ReviewMix::BeforeReviews => Box::new(SizedChain::new(day_learning_iter, reviews_iter)),
        ReviewMix::MixWithReviews => Box::new(Intersperser::new(reviews_iter, day_learning_iter)),
    }
}

fn merge_new(
    review_iter: impl ExactSizeIterator<Item = MainQueueEntry> + 'static,
    new: Vec<NewCard>,
    mode: ReviewMix,
) -> Box<dyn ExactSizeIterator<Item = MainQueueEntry>> {
    let new_iter = new.into_iter().map(Into::into);

    match mode {
        ReviewMix::BeforeReviews => Box::new(SizedChain::new(new_iter, review_iter)),
        ReviewMix::AfterReviews => Box::new(SizedChain::new(review_iter, new_iter)),
        ReviewMix::MixWithReviews => Box::new(Intersperser::new(review_iter, new_iter)),
    }
}

fn sort_learning(mut learning: Vec<DueCard>) -> VecDeque<LearningQueueEntry> {
    learning.sort_unstable_by(|a, b| a.due.cmp(&b.due));
    learning.into_iter().map(LearningQueueEntry::from).collect()
}

impl Collection {
    pub(crate) fn build_queues(&mut self, deck_id: DeckId) -> Result<CardQueues> {
        let now = TimestampSecs::now();
        let timing = self.timing_for_timestamp(now)?;
        let decks = self.storage.deck_with_children(deck_id)?;
        // need full map, since filtered decks may contain cards from decks
        // outside tree
        let deck_map = self.storage.get_decks_map()?;
        let config = self.storage.get_deck_config_map()?;
        let sort_options = decks[0]
            .config_id()
            .and_then(|config_id| config.get(&config_id))
            .map(|config| QueueSortOptions {
                new_order: config.inner.new_card_sort_order(),
                new_gather_priority: config.inner.new_card_gather_priority(),
                review_order: config.inner.review_order(),
                day_learn_mix: config.inner.interday_learning_mix(),
                new_review_mix: config.inner.new_mix(),
            })
            .unwrap_or_else(|| {
                // filtered decks do not space siblings
                QueueSortOptions {
                    new_order: NewCardSortOrder::LowestPosition,
                    ..Default::default()
                }
            });

        // fetch remaining limits, and cap to selected deck limits so that we don't
        // do more work than necessary
        let mut remaining = remaining_limits_map(decks.iter(), &config, timing.days_elapsed, true);
        let selected_deck_limits_at_start = *remaining.get(&deck_id).unwrap();
        let mut selected_deck_limits = selected_deck_limits_at_start;
        for limit in remaining.values_mut() {
            limit.cap_to(selected_deck_limits);
        }

        self.storage.update_active_decks(&decks[0])?;
        let mut queues = QueueBuilder::new(sort_options.clone());

        let get_bury_mode = |home_deck: DeckId| {
            deck_map
                .get(&home_deck)
                .and_then(|deck| deck.config_id())
                .and_then(|config_id| config.get(&config_id))
                .map(|config| BuryMode {
                    bury_new: config.inner.bury_new,
                    bury_reviews: config.inner.bury_reviews,
                })
                .unwrap_or_default()
        };

        // intraday cards first, noting down any notes that will need burying
        self.storage
            .for_each_intraday_card_in_active_decks(timing.next_day_at, |card| {
                let bury = get_bury_mode(card.current_deck_id);
                queues.add_intraday_learning_card(card, bury)
            })?;

        // interday learning, then reviews
        let mut add_due_cards = |kind: DueCardKind| -> Result<()> {
            if selected_deck_limits.review != 0 {
                self.storage.for_each_due_card_in_active_decks(
                    timing.days_elapsed,
                    sort_options.review_order,
                    kind,
                    |card| {
                        if selected_deck_limits.review == 0 {
                            return false;
                        }
                        let bury = get_bury_mode(card.original_deck_id.or(card.current_deck_id));
                        let limits = remaining.get_mut(&card.current_deck_id).unwrap();
                        if limits.review != 0 && queues.add_due_card(card, bury) {
                            selected_deck_limits.review -= 1;
                            limits.review -= 1;
                        }

                        true
                    },
                )?;
            }
            Ok(())
        };
        add_due_cards(DueCardKind::Learning)?;
        add_due_cards(DueCardKind::Review)?;

        // cap new cards to the remaining review limit
        for limit in remaining.values_mut() {
            limit.new = limit.new.min(limit.review).min(selected_deck_limits.review);
        }
        selected_deck_limits.new = selected_deck_limits.new.min(selected_deck_limits.review);

        // new cards last
        let can_exit_early = sort_options.new_gather_priority == NewCardGatherPriority::Deck;
        let reverse = sort_options.new_gather_priority == NewCardGatherPriority::HighestPosition;
        for deck in &decks {
            if can_exit_early && selected_deck_limits.new == 0 {
                break;
            }
            let limit = remaining.get_mut(&deck.id).unwrap();
            if limit.new > 0 {
                self.storage
                    .for_each_new_card_in_deck(deck.id, reverse, |card| {
                        let bury = get_bury_mode(card.original_deck_id.or(deck.id));
                        if limit.new != 0 {
                            if queues.add_new_card(card, bury) {
                                limit.new -= 1;
                                selected_deck_limits.new =
                                    selected_deck_limits.new.saturating_sub(1);
                            }

                            true
                        } else {
                            false
                        }
                    })?;
            }
        }

        let final_limits = RemainingLimits {
            new: selected_deck_limits_at_start
                .new
                .min(selected_deck_limits.review),
            ..selected_deck_limits_at_start
        };
        let queues = queues.build(
            final_limits,
            self.learn_ahead_secs() as i64,
            timing.days_elapsed,
        );

        Ok(queues)
    }
}
