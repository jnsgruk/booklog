mod books;
mod readings;
pub mod stats;
pub mod timeline_refresh;

pub use books::BookService;
pub use readings::ReadingService;
pub use stats::StatsInvalidator;
pub use timeline_refresh::TimelineInvalidator;

use std::sync::Arc;

use tracing::warn;

use crate::domain::errors::RepositoryError;
use crate::domain::ids::UserId;
use crate::domain::repositories::TimelineEventRepository;

/// Generates a service struct with a `create` method that inserts via the
/// repository and then records a timeline event (fire-and-forget).
///
/// Use this for entities whose `to_timeline_event()` method needs `&self`
/// and a `UserId`. For entities that need enrichment or cross-repo lookups,
/// write the service by hand.
///
/// # Example
/// ```ignore
/// define_simple_service!(AuthorService, AuthorRepository, Author, NewAuthor, "author");
/// ```
macro_rules! define_simple_service {
    ($service:ident, $repo_trait:path, $entity:ty, $new_entity:ty, $entity_name:literal) => {
        #[derive(Clone)]
        pub struct $service {
            repo: Arc<dyn $repo_trait>,
            timeline_repo: Arc<dyn TimelineEventRepository>,
        }

        impl $service {
            pub fn new(
                repo: Arc<dyn $repo_trait>,
                timeline_repo: Arc<dyn TimelineEventRepository>,
            ) -> Self {
                Self {
                    repo,
                    timeline_repo,
                }
            }

            pub async fn create(
                &self,
                new: $new_entity,
                user_id: UserId,
            ) -> Result<$entity, RepositoryError> {
                let entity = self.repo.insert(new).await?;
                if let Err(err) = self
                    .timeline_repo
                    .insert(entity.to_timeline_event(user_id))
                    .await
                {
                    warn!(
                        error = %err,
                        id = %entity.id,
                        concat!("failed to record ", $entity_name, " timeline event"),
                    );
                }
                Ok(entity)
            }
        }
    };
}

use crate::domain::authors::{Author, NewAuthor};
use crate::domain::genres::{Genre, NewGenre};
use crate::domain::repositories::{AuthorRepository, GenreRepository};

define_simple_service!(AuthorService, AuthorRepository, Author, NewAuthor, "author");
define_simple_service!(GenreService, GenreRepository, Genre, NewGenre, "genre");
