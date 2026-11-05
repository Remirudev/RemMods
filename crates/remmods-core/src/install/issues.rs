//! Issue collection: one line per severity and code, with the first path and a count.

use super::model::{InstallIssue, IssueCode, IssueSeverity};

#[derive(Debug, Default)]
pub(crate) struct Issues {
    list: Vec<InstallIssue>,
}

impl Issues {
    pub(crate) fn add(&mut self, severity: IssueSeverity, code: IssueCode, path: &str) {
        self.add_count(severity, code, path, 1);
    }

    pub(crate) fn add_count(
        &mut self,
        severity: IssueSeverity,
        code: IssueCode,
        path: &str,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        let existing = self
            .list
            .iter_mut()
            .find(|issue| issue.severity == severity && issue.code == code);
        match existing {
            Some(issue) => issue.count = issue.count.saturating_add(count),
            None => self.list.push(InstallIssue {
                severity,
                code,
                path: path.to_owned(),
                count,
            }),
        }
    }

    pub(crate) fn block(&mut self, code: IssueCode, path: &str) {
        self.add(IssueSeverity::Block, code, path);
    }

    pub(crate) fn warn(&mut self, code: IssueCode, path: &str) {
        self.add(IssueSeverity::Warn, code, path);
    }

    pub(crate) fn info(&mut self, code: IssueCode, path: &str) {
        self.add(IssueSeverity::Info, code, path);
    }

    pub(crate) fn blocked(&self) -> bool {
        self.list
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Block)
    }

    /// Block first, then Warn, then Info; insertion order within a severity.
    pub(crate) fn into_list(self) -> Vec<InstallIssue> {
        let mut list = self.list;
        list.sort_by_key(|issue| match issue.severity {
            IssueSeverity::Block => 0,
            IssueSeverity::Warn => 1,
            IssueSeverity::Info => 2,
        });
        list
    }
}

pub(crate) fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}
