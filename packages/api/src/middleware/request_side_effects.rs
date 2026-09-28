use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PostCommitSideEffect {
    DeleteKnowledgeVectors { asset_id: Uuid },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RollbackSideEffect {
    DeleteKnowledgeStorageObject {
        object_key: String,
        school_id: Uuid,
    },
}

/// External effects that must be ordered relative to the request-scoped
/// PostgreSQL transaction. Handlers only register intent; auth middleware drains
/// the matching queue after the transaction has definitively committed or
/// rolled back.
#[derive(Debug, Default)]
pub struct RequestSideEffects {
    post_commit: Mutex<Vec<PostCommitSideEffect>>,
    rollback: Mutex<Vec<RollbackSideEffect>>,
}

impl RequestSideEffects {
    pub fn delete_knowledge_vectors_after_commit(&self, asset_id: Uuid) {
        self.post_commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(PostCommitSideEffect::DeleteKnowledgeVectors { asset_id });
    }

    pub fn delete_knowledge_storage_on_rollback(
        &self,
        object_key: String,
        school_id: Uuid,
    ) {
        self.rollback
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(RollbackSideEffect::DeleteKnowledgeStorageObject {
                object_key,
                school_id,
            });
    }

    pub(crate) fn take_post_commit(&self) -> Vec<PostCommitSideEffect> {
        std::mem::take(
            &mut *self
                .post_commit
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }

    pub(crate) fn take_rollback(&self) -> Vec<RollbackSideEffect> {
        std::mem::take(
            &mut *self
                .rollback
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_and_rollback_queues_are_independent_and_drained_once() {
        let queue = RequestSideEffects::default();
        let asset_id = Uuid::new_v4();
        let school_id = Uuid::new_v4();

        queue.delete_knowledge_vectors_after_commit(asset_id);
        queue.delete_knowledge_storage_on_rollback("school/object.pdf".into(), school_id);

        assert_eq!(
            queue.take_post_commit(),
            vec![PostCommitSideEffect::DeleteKnowledgeVectors { asset_id }]
        );
        assert!(queue.take_post_commit().is_empty());

        assert_eq!(
            queue.take_rollback(),
            vec![RollbackSideEffect::DeleteKnowledgeStorageObject {
                object_key: "school/object.pdf".into(),
                school_id,
            }]
        );
        assert!(queue.take_rollback().is_empty());
    }
}
