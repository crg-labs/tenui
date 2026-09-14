#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cursor {
    pub anchor: usize,
    pub head: usize,
}

impl Cursor {
    pub fn point(pos: usize) -> Self {
        Self { anchor: pos, head: pos }
    }

    pub fn is_selection(&self) -> bool {
        self.anchor != self.head
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }
}

pub struct CursorSet {
    cursors: Vec<Cursor>,
}

impl CursorSet {
    pub fn new(initial_pos: usize) -> Self {
        Self {
            cursors: vec![Cursor::point(initial_pos)],
        }
    }

    pub fn len(&self) -> usize {
        self.cursors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cursors.is_empty()
    }

    pub fn add_cursor(&mut self, cursor: Cursor) {
        self.cursors.push(cursor);
        self.normalize();
    }

    /// Sorts and merges overlapping or abutting selections
    pub fn normalize(&mut self) {
        if self.cursors.len() <= 1 {
            return;
        }

        self.cursors.sort_by_key(|c| c.start());
        let mut merged = Vec::with_capacity(self.cursors.len());
        let mut current = self.cursors[0];

        for next in self.cursors.drain(1..) {
            if next.start() <= current.end() {
                // Overlap: coalesce
                let new_start = current.start();
                let new_end = current.end().max(next.end());
                let head = if current.head >= current.anchor {
                    new_end
                } else {
                    new_start
                };
                let anchor = if current.head >= current.anchor {
                    new_start
                } else {
                    new_end
                };
                current = Cursor { anchor, head };
            } else {
                merged.push(current);
                current = next;
            }
        }
        merged.push(current);
        self.cursors = merged;
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Cursor> {
        self.cursors.iter()
    }

    /// Construct multi-row column / rectangular block selection across buffer lines.
    pub fn from_rectangular_block(
        lines: &[&str],
        start_col: usize,
        start_row: usize,
        end_col: usize,
        end_row: usize,
    ) -> Self {
        let r_min = start_row.min(end_row);
        let r_max = start_row.max(end_row);
        let c_min = start_col.min(end_col);
        let c_max = start_col.max(end_col);

        let mut cursors = Vec::new();
        let mut byte_offset = 0;

        for (row_idx, line) in lines.iter().enumerate() {
            let line_len = line.len();
            if row_idx >= r_min && row_idx <= r_max {
                let col_start = c_min.min(line_len);
                let col_end = c_max.min(line_len);
                let anchor = byte_offset + col_start;
                let head = byte_offset + col_end;
                cursors.push(Cursor { anchor, head });
            }
            byte_offset += line_len + 1; // +1 for newline
        }

        if cursors.is_empty() {
            CursorSet::new(0)
        } else {
            let mut set = CursorSet { cursors };
            set.normalize();
            set
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEditDelta {
    pub range: (usize, usize),
    pub replaced_text: String,
    pub inserted_text: String,
}

#[derive(Debug, Clone)]
pub struct UndoTreeNode {
    pub id: usize,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub deltas: Vec<TextEditDelta>,
    pub cursors: Vec<Cursor>,
}

#[derive(Debug, Clone)]
pub struct BranchingUndoTree {
    nodes: Vec<UndoTreeNode>,
    pub current_node: usize,
}

impl BranchingUndoTree {
    pub fn new(initial_cursors: Vec<Cursor>) -> Self {
        let root = UndoTreeNode {
            id: 0,
            parent: None,
            children: Vec::new(),
            deltas: Vec::new(),
            cursors: initial_cursors,
        };
        Self {
            nodes: vec![root],
            current_node: 0,
        }
    }

    pub fn commit_edit(&mut self, deltas: Vec<TextEditDelta>, cursors: Vec<Cursor>) -> usize {
        let new_id = self.nodes.len();
        let new_node = UndoTreeNode {
            id: new_id,
            parent: Some(self.current_node),
            children: Vec::new(),
            deltas,
            cursors,
        };
        self.nodes.push(new_node);
        self.nodes[self.current_node].children.push(new_id);
        self.current_node = new_id;
        new_id
    }

    pub fn undo(&mut self) -> Option<(&[TextEditDelta], &[Cursor])> {
        let parent_id = self.nodes[self.current_node].parent?;
        let current = &self.nodes[self.current_node];
        self.current_node = parent_id;
        Some((&current.deltas, &self.nodes[parent_id].cursors))
    }

    pub fn redo(&mut self, branch_index: usize) -> Option<(&[TextEditDelta], &[Cursor])> {
        let current = &self.nodes[self.current_node];
        if branch_index >= current.children.len() {
            return None;
        }
        let child_id = current.children[branch_index];
        self.current_node = child_id;
        let child = &self.nodes[child_id];
        Some((&child.deltas, &child.cursors))
    }

    pub fn current_branch_count(&self) -> usize {
        self.nodes[self.current_node].children.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_cursor_overlapping_coalesce() {
        let mut cursors = CursorSet::new(5);
        // Add overlapping cursors
        cursors.add_cursor(Cursor { anchor: 4, head: 8 });
        cursors.add_cursor(Cursor { anchor: 7, head: 12 });

        // Invariant: Overlapping selections coalesce into single span: [4, 12]
        let list: Vec<_> = cursors.iter().copied().collect();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].start(), 4);
        assert_eq!(list[0].end(), 12);
    }

    #[test]
    fn test_rectangular_block_selection() {
        let lines = ["const foo = 10;", "const bar = 20;", "const baz = 30;"];
        let block_set = CursorSet::from_rectangular_block(&lines, 6, 0, 9, 2);
        let list: Vec<_> = block_set.iter().copied().collect();
        assert_eq!(list.len(), 3);
        // "foo", "bar", "baz"
        assert_eq!(list[0].start(), 6);
        assert_eq!(list[0].end(), 9);
    }

    #[test]
    fn test_branching_undo_tree() {
        let mut tree = BranchingUndoTree::new(vec![Cursor::point(0)]);

        // Rev 1
        tree.commit_edit(
            vec![TextEditDelta {
                range: (0, 0),
                replaced_text: "".into(),
                inserted_text: "foo".into(),
            }],
            vec![Cursor::point(3)],
        );
        assert_eq!(tree.current_node, 1);

        // Rev 2
        tree.commit_edit(
            vec![TextEditDelta {
                range: (3, 3),
                replaced_text: "".into(),
                inserted_text: "bar".into(),
            }],
            vec![Cursor::point(6)],
        );
        assert_eq!(tree.current_node, 2);

        // Undo -> back to Rev 1
        let (deltas, cursors) = tree.undo().unwrap();
        assert_eq!(deltas[0].inserted_text, "bar");
        assert_eq!(cursors[0].head, 3);
        assert_eq!(tree.current_node, 1);

        // Branch new edit from Rev 1 -> Rev 3 (sibling of Rev 2)
        tree.commit_edit(
            vec![TextEditDelta {
                range: (3, 3),
                replaced_text: "".into(),
                inserted_text: "qux".into(),
            }],
            vec![Cursor::point(6)],
        );
        assert_eq!(tree.current_node, 3);

        // Undo -> back to Rev 1
        tree.undo();
        assert_eq!(tree.current_node, 1);
        assert_eq!(tree.current_branch_count(), 2); // Rev 2 and Rev 3!

        // Redo branch 0 (Rev 2: "bar")
        let (deltas, _) = tree.redo(0).unwrap();
        assert_eq!(deltas[0].inserted_text, "bar");
        assert_eq!(tree.current_node, 2);

        // Undo then Redo branch 1 (Rev 3: "qux")
        tree.undo();
        let (deltas, _) = tree.redo(1).unwrap();
        assert_eq!(deltas[0].inserted_text, "qux");
        assert_eq!(tree.current_node, 3);
    }
}
