// SPDX-License-Identifier: AGPL-3.0-or-later
//! The append-only edit log.
//!
//! A document being edited by a person and one or more agents at the same time
//! cannot be a file that each of them rewrites. So the content is not the
//! authority here — the log is. Saving appends operations to the end of the
//! file; the block tree is what you get by replaying them.
//!
//! # Why this converges
//!
//! Materialisation is a pure function of the *set* of operations: the log is
//! sorted by operation id and replayed deterministically, so two replicas
//! holding the same set produce byte-identical documents regardless of the
//! order they received them in. Merging is therefore just a set union, which is
//! commutative, associative and idempotent — the three properties that make
//! this a CRDT rather than a merge algorithm that usually works.
//!
//! Ordering among siblings uses RGA: an insert names the node it goes after,
//! and concurrent inserts at the same anchor are ordered by descending id, so
//! every replica breaks the tie the same way.
//!
//! # What this does and does not merge
//!
//! Granularity is the **block**. Two edits to different paragraphs always
//! merge. Two edits to the *same* paragraph resolve last-writer-wins by
//! Lamport clock, with the actor id breaking exact ties — meaning one of them
//! is discarded rather than the two being woven together character by
//! character. Character-level merging needs a text CRDT per paragraph, which is
//! a much larger object with a per-character storage cost; block granularity is
//! the right trade for documents whose paragraphs are edited whole, and it is
//! stated here rather than discovered later.
//!
//! Every operation carries who performed it — including *which model*, when the
//! actor is an agent — so attribution and history come from the storage layer
//! instead of being maintained beside it.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::doc::Block;

use super::AdfError;
use super::codec;
use super::wire::{Reader, Writer};

/// Identifies one operation, and thereby the node it creates.
///
/// Ordered by `counter` then `actor`: the Lamport clock gives causality, and
/// the actor id makes the order total so that concurrent operations still sort
/// deterministically on every replica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpId {
    pub counter: u64,
    pub actor: u64,
}

impl OpId {
    pub const ROOT: OpId = OpId {
        counter: 0,
        actor: 0,
    };

    pub fn is_root(&self) -> bool {
        *self == OpId::ROOT
    }
}

/// Who performed an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    /// Stable id, unique within a document.
    pub id: u64,
    /// Display name — a person, or a model identifier such as `claude-opus-5`.
    pub name: String,
    /// Whether this actor is an agent rather than a person. Kept explicit
    /// because "who changed this" has a different weight in review depending on
    /// the answer, and a name alone does not reliably say.
    pub is_agent: bool,
}

/// One edit.
#[derive(Debug, Clone)]
pub enum Change {
    /// Insert a block under `parent`, positioned after `left` (or first when
    /// `left` is `None`).
    Insert {
        parent: OpId,
        left: Option<OpId>,
        block: Block,
    },
    /// Tombstone a node. Its children go with it.
    Delete { target: OpId },
    /// Replace a node's content, last-writer-wins.
    Replace { target: OpId, block: Block },
}

/// An operation: an identified, attributed, timestamped change.
#[derive(Debug, Clone)]
pub struct Op {
    pub id: OpId,
    /// Actor id, matching [`Actor::id`].
    pub author: u64,
    /// Milliseconds since the Unix epoch. Advisory only — ordering uses the
    /// Lamport clock, because wall clocks on different machines disagree and
    /// trusting them would make merges depend on whose clock was fast.
    pub timestamp: u64,
    pub change: Change,
}

// Change tags.
const TAG_INSERT: u8 = 1;
const TAG_DELETE: u8 = 2;
const TAG_REPLACE: u8 = 3;

/// The log, plus the actors that wrote it.
#[derive(Debug, Clone, Default)]
pub struct OpLog {
    ops: BTreeMap<OpId, Op>,
    actors: BTreeMap<u64, Actor>,
}

impl OpLog {
    pub fn new() -> OpLog {
        OpLog::default()
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn ops(&self) -> impl DoubleEndedIterator<Item = &Op> {
        self.ops.values()
    }

    pub fn actors(&self) -> impl Iterator<Item = &Actor> {
        self.actors.values()
    }

    pub fn actor(&self, id: u64) -> Option<&Actor> {
        self.actors.get(&id)
    }

    pub fn register_actor(&mut self, actor: Actor) {
        self.actors.insert(actor.id, actor);
    }

    /// The next counter value this replica should use.
    ///
    /// One past the highest seen, which is what keeps a new operation ordered
    /// after everything it could have been aware of.
    pub fn next_counter(&self) -> u64 {
        self.ops.keys().next_back().map_or(1, |id| id.counter + 1)
    }

    /// Append a change by `author`, allocating its id.
    pub fn push(&mut self, author: u64, timestamp: u64, change: Change) -> OpId {
        let id = OpId {
            counter: self.next_counter(),
            actor: author,
        };
        self.ops.insert(id, Op {
            id,
            author,
            timestamp,
            change,
        });
        id
    }

    /// Add an operation that came from elsewhere. Idempotent: an operation
    /// already present is ignored, so replaying a log twice is harmless.
    pub fn apply(&mut self, op: Op) {
        self.ops.entry(op.id).or_insert(op);
    }

    /// Union another log into this one.
    ///
    /// This is the whole merge algorithm. It is a set union because operation
    /// ids are globally unique and operations are immutable.
    pub fn merge(&mut self, other: &OpLog) {
        for op in other.ops.values() {
            self.apply(op.clone());
        }
        for actor in other.actors.values() {
            self.actors
                .entry(actor.id)
                .or_insert_with(|| actor.clone());
        }
    }

    /// Replay the log into the blocks under `parent`.
    pub fn materialize(&self, parent: OpId) -> Vec<Block> {
        self.order(parent)
            .into_iter()
            .filter_map(|id| self.resolve(id))
            .collect()
    }

    /// Replay into blocks, each paired with the node id that produced it, for
    /// callers that need to address nodes afterwards — an editor selection, or
    /// an agent citing what it just changed.
    pub fn materialize_with_ids(&self, parent: OpId) -> Vec<(OpId, Block)> {
        self.order(parent)
            .into_iter()
            .filter_map(|id| self.resolve(id).map(|block| (id, block)))
            .collect()
    }

    /// Tombstoned nodes, so a caller can distinguish "never existed" from
    /// "deleted" — the two need different answers when resolving a citation.
    pub fn deleted(&self) -> HashSet<OpId> {
        self.ops
            .values()
            .filter_map(|op| match &op.change {
                Change::Delete { target } => Some(*target),
                _ => None,
            })
            .collect()
    }

    /// Who last changed a node, and when.
    pub fn attribution(&self, node: OpId) -> Option<(&Actor, u64)> {
        let winner = self.winning_op(node)?;
        self.actors
            .get(&winner.author)
            .map(|actor| (actor, winner.timestamp))
    }

    /// The operation whose content currently wins for `node`.
    ///
    /// The insert, unless a later `Replace` beats it. "Later" is by operation
    /// id, so the Lamport counter decides and the actor id breaks exact ties —
    /// never the wall clock.
    fn winning_op(&self, node: OpId) -> Option<&Op> {
        let mut winner = self.ops.get(&node)?;
        for op in self.ops.values() {
            if let Change::Replace { target, .. } = &op.change
                && *target == node
                && op.id > winner.id
            {
                winner = op;
            }
        }
        Some(winner)
    }

    /// The current content of a node, or `None` if it is deleted.
    fn resolve(&self, node: OpId) -> Option<Block> {
        if self.deleted().contains(&node) {
            return None;
        }
        match &self.winning_op(node)?.change {
            Change::Insert { block, .. } | Change::Replace { block, .. } => Some(block.clone()),
            Change::Delete { .. } => None,
        }
    }

    /// RGA ordering of `parent`'s live children.
    fn order(&self, parent: OpId) -> Vec<OpId> {
        // Inserts under this parent, oldest first. Replaying in id order is
        // what makes the result a function of the set rather than of arrival.
        let inserts: Vec<(&OpId, Option<OpId>)> = self
            .ops
            .values()
            .filter_map(|op| match &op.change {
                Change::Insert { parent: p, left, .. } if *p == parent => Some((&op.id, *left)),
                _ => None,
            })
            .collect();

        let mut sequence: Vec<OpId> = Vec::with_capacity(inserts.len());
        for (id, left) in inserts {
            // Position just after the anchor. An anchor that is not present —
            // a node deleted and garbage-collected, or an op that arrived
            // before the one it references — degrades to the front rather than
            // dropping the insert.
            let mut at = match left {
                None => 0,
                Some(anchor) => sequence
                    .iter()
                    .position(|node| *node == anchor)
                    .map_or(0, |index| index + 1),
            };
            // Concurrent inserts at the same anchor: higher id goes first, and
            // every replica agrees because ids are totally ordered.
            while at < sequence.len() && sequence[at] > *id {
                at += 1;
            }
            sequence.insert(at, *id);
        }

        let deleted = self.deleted();
        sequence.retain(|node| !deleted.contains(node));
        sequence
    }

    // ------------------------------------------------------------------
    // Serialisation
    // ------------------------------------------------------------------

    /// Encode the whole log.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Writer::new();

        out.usize(self.actors.len());
        for actor in self.actors.values() {
            out.u64(actor.id);
            out.bytes_with_len(actor.name.as_bytes());
            out.u8(u8::from(actor.is_agent));
        }

        out.usize(self.ops.len());
        for op in self.ops.values() {
            encode_op(&mut out, op);
        }
        out.bytes
    }

    /// Encode only operations after `since`, which is what an append writes.
    ///
    /// This is why the log is last in the file: appending an edit copies the
    /// new operations and nothing else, so saving a large document after a
    /// one-word change stays proportional to the change.
    pub fn encode_since(&self, since: Option<OpId>) -> Vec<u8> {
        let mut out = Writer::new();
        let new: Vec<&Op> = self
            .ops
            .values()
            .filter(|op| since.is_none_or(|mark| op.id > mark))
            .collect();

        out.usize(0); // no actor records in an incremental segment
        out.usize(new.len());
        for op in new {
            encode_op(&mut out, op);
        }
        out.bytes
    }

    /// Parse a log region.
    ///
    /// The region is a *sequence* of segments, not one record: every append
    /// writes a fresh segment on the end rather than rewriting what is already
    /// there, so reading continues until the bytes run out. Because merging is
    /// a set union, segments that repeat an operation are harmless.
    pub fn parse(bytes: &[u8]) -> Result<OpLog, AdfError> {
        let mut reader = Reader::new(bytes);
        let mut log = OpLog::new();

        while !reader.is_empty() {
            let actor_count = reader.count()?;
            for _ in 0..actor_count {
                let id = reader.u64()?;
                let name = std::str::from_utf8(reader.bytes_with_len()?)
                    .map_err(|_| AdfError::Malformed("actor name is not UTF-8"))?
                    .to_string();
                let is_agent = reader.u8()? != 0;
                log.register_actor(Actor { id, name, is_agent });
            }

            let op_count = reader.count()?;
            for _ in 0..op_count {
                let op = decode_op(&mut reader)?;
                log.ops.entry(op.id).or_insert(op);
            }
        }
        Ok(log)
    }
}

fn encode_op(out: &mut Writer, op: &Op) {
    out.u64(op.id.counter);
    out.u64(op.id.actor);
    out.u64(op.author);
    out.u64(op.timestamp);

    match &op.change {
        Change::Insert {
            parent,
            left,
            block,
        } => {
            out.u8(TAG_INSERT);
            out.u64(parent.counter);
            out.u64(parent.actor);
            out.option(*left, |w, id| {
                w.u64(id.counter);
                w.u64(id.actor);
            });
            out.bytes_with_len(&codec::encode_block_standalone(block));
        }
        Change::Delete { target } => {
            out.u8(TAG_DELETE);
            out.u64(target.counter);
            out.u64(target.actor);
        }
        Change::Replace { target, block } => {
            out.u8(TAG_REPLACE);
            out.u64(target.counter);
            out.u64(target.actor);
            out.bytes_with_len(&codec::encode_block_standalone(block));
        }
    }
}

fn decode_op(reader: &mut Reader<'_>) -> Result<Op, AdfError> {
    let id = OpId {
        counter: reader.u64()?,
        actor: reader.u64()?,
    };
    let author = reader.u64()?;
    let timestamp = reader.u64()?;

    let change = match reader.u8()? {
        TAG_INSERT => {
            let parent = OpId {
                counter: reader.u64()?,
                actor: reader.u64()?,
            };
            let left = reader.option(|r| {
                Ok(OpId {
                    counter: r.u64()?,
                    actor: r.u64()?,
                })
            })?;
            let block = codec::decode_block_standalone(reader.bytes_with_len()?)?;
            Change::Insert {
                parent,
                left,
                block,
            }
        }
        TAG_DELETE => Change::Delete {
            target: OpId {
                counter: reader.u64()?,
                actor: reader.u64()?,
            },
        },
        TAG_REPLACE => {
            let target = OpId {
                counter: reader.u64()?,
                actor: reader.u64()?,
            };
            let block = codec::decode_block_standalone(reader.bytes_with_len()?)?;
            Change::Replace { target, block }
        }
        _ => return Err(AdfError::Malformed("unknown operation tag")),
    };

    Ok(Op {
        id,
        author,
        timestamp,
        change,
    })
}

/// Build a log that reproduces an existing document, so an imported file can be
/// edited collaboratively from the moment it is opened.
pub fn from_blocks(blocks: &[Block], author: u64, timestamp: u64) -> OpLog {
    let mut log = OpLog::new();
    let mut left = None;
    for block in blocks {
        left = Some(log.push(author, timestamp, Change::Insert {
            parent: OpId::ROOT,
            left,
            block: block.clone(),
        }));
    }
    log
}

/// Count operations per actor, for a history view.
pub fn edit_counts(log: &OpLog) -> HashMap<u64, usize> {
    let mut counts = HashMap::new();
    for op in log.ops() {
        *counts.entry(op.author).or_insert(0) += 1;
    }
    counts
}
