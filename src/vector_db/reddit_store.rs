use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use arrow::array::{
    Float32Array, RecordBatch, RecordBatchIterator, as_boolean_array, as_primitive_array,
    as_string_array,
};
use arrow::datatypes::{Int32Type, Int64Type, TimestampSecondType, UInt32Type};
use arrow_array::Array;
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use chrono::DateTime;
use futures::TryStreamExt;
use lance::dataset::optimize::compact_files;
use lance::dataset::{Dataset, WriteMode, WriteParams};
use lance_index::scalar::FullTextSearchQuery;
use lance_index::scalar::ScalarIndexParams;
use lance_index::scalar::inverted::InvertedIndexParams;
use lance_index::scalar::inverted::query::{FtsQuery, MatchQuery, Operator};
use lance_index::vector::hnsw::builder::HnswBuildParams;
use lance_index::vector::ivf::IvfBuildParams;
use lance_index::vector::pq::PQBuildParams;
use lance_index::{DatasetIndexExt, IndexType};
use tokio::sync::{mpsc, oneshot};
use tokio::time::interval;
use tracing::{error, info};

use crate::storage::types::{Direction, EMBEDDING_DIM, VectorQuery};
use crate::storage::{BATCH_SIZE, FLUSH_INTERVAL_SECS, OPEN_PATHS};
use arrow_array::builder::{
    BooleanBuilder, FixedSizeListBuilder, Float32Builder, Int32Builder, Int64Builder,
    StringBuilder, TimestampSecondBuilder, UInt32Builder,
};
use lance::dataset::scanner::ColumnOrdering;

// -- Reddit storage types -------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Post,
    Comment,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Post => "post",
            Kind::Comment => "comment",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub name: String,
    pub title: String,
    pub content: String,
    pub subreddit: String,
    pub subreddit_id: String,
    pub author: String,
    pub score: i32,
    pub num_comments: i32,
    pub over_18: bool,
    pub permalink: String,
    pub url: String,
    pub subscribers: i64,
    #[serde(skip)]
    pub embedding: Vec<f32>,
    pub created_utc: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub name: String,
    pub title: String,
    pub content: String,
    pub subreddit: String,
    pub subreddit_id: String,
    pub author: String,
    pub score: i32,
    pub num_comments: i32,
    pub over_18: bool,
    pub permalink: String,
    pub url: String,
    pub post_id: String,
    pub parent_id: String,
    pub depth: u32,
    pub subscribers: i64,
    #[serde(skip)]
    pub embedding: Vec<f32>,
    pub created_utc: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Item {
    Post(Post),
    Comment(Comment),
}

impl Item {
    pub fn name(&self) -> &str {
        match self {
            Item::Post(p) => &p.name,
            Item::Comment(c) => &c.name,
        }
    }

    pub fn kind(&self) -> Kind {
        match self {
            Item::Post(_) => Kind::Post,
            Item::Comment(_) => Kind::Comment,
        }
    }

    fn title(&self) -> &str {
        match self {
            Item::Post(p) => &p.title,
            Item::Comment(c) => &c.title,
        }
    }

    fn content(&self) -> &str {
        match self {
            Item::Post(p) => &p.content,
            Item::Comment(c) => &c.content,
        }
    }

    fn subreddit(&self) -> &str {
        match self {
            Item::Post(p) => &p.subreddit,
            Item::Comment(c) => &c.subreddit,
        }
    }

    fn subreddit_id(&self) -> &str {
        match self {
            Item::Post(p) => &p.subreddit_id,
            Item::Comment(c) => &c.subreddit_id,
        }
    }

    fn author(&self) -> &str {
        match self {
            Item::Post(p) => &p.author,
            Item::Comment(c) => &c.author,
        }
    }

    fn score(&self) -> i32 {
        match self {
            Item::Post(p) => p.score,
            Item::Comment(c) => c.score,
        }
    }

    fn num_comments(&self) -> i32 {
        match self {
            Item::Post(p) => p.num_comments,
            Item::Comment(c) => c.num_comments,
        }
    }

    fn over_18(&self) -> bool {
        match self {
            Item::Post(p) => p.over_18,
            Item::Comment(c) => c.over_18,
        }
    }

    fn permalink(&self) -> &str {
        match self {
            Item::Post(p) => &p.permalink,
            Item::Comment(c) => &c.permalink,
        }
    }

    fn url(&self) -> &str {
        match self {
            Item::Post(p) => &p.url,
            Item::Comment(c) => &c.url,
        }
    }

    fn post_id(&self) -> Option<&str> {
        match self {
            Item::Post(_) => None,
            Item::Comment(c) => Some(&c.post_id),
        }
    }

    fn parent_id(&self) -> Option<&str> {
        match self {
            Item::Post(_) => None,
            Item::Comment(c) => Some(&c.parent_id),
        }
    }

    fn depth(&self) -> Option<u32> {
        match self {
            Item::Post(_) => None,
            Item::Comment(c) => Some(c.depth),
        }
    }

    fn subscribers(&self) -> i64 {
        match self {
            Item::Post(p) => p.subscribers,
            Item::Comment(c) => c.subscribers,
        }
    }

    fn embedding(&self) -> &[f32] {
        match self {
            Item::Post(p) => &p.embedding,
            Item::Comment(c) => &c.embedding,
        }
    }

    fn created_utc(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            Item::Post(p) => p.created_utc,
            Item::Comment(c) => c.created_utc,
        }
    }

    fn arrow_schema() -> Schema {
        Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("kind", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("search_text", DataType::Utf8, false),
            Field::new("subreddit", DataType::Utf8, false),
            Field::new("subreddit_id", DataType::Utf8, false),
            Field::new("author", DataType::Utf8, false),
            Field::new("score", DataType::Int32, false),
            Field::new("num_comments", DataType::Int32, false),
            Field::new("over_18", DataType::Boolean, false),
            Field::new("permalink", DataType::Utf8, false),
            Field::new("url", DataType::Utf8, false),
            Field::new("post_id", DataType::Utf8, true),
            Field::new("parent_id", DataType::Utf8, true),
            Field::new("depth", DataType::UInt32, true),
            Field::new("subscribers", DataType::Int64, false),
            crate::storage::types::embedding_field("embedding"),
            Field::new(
                "created_utc",
                DataType::Timestamp(TimeUnit::Second, None),
                false,
            ),
        ])
    }
}

/// Item with cosine similarity from a vector search query.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredItem {
    pub item: Item,
    pub similarity: Option<f32>,
}

/// Result of a search query with pagination metadata.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub items: Vec<ScoredItem>,
    pub total: u64,
}

/// A post with all its comments.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub post: Item,
    pub comments: Vec<Item>,
}

// -- Commands --------------------------------------------------------------

/// Commands sent to the background actor.
enum Command {
    AppendItems(Vec<Item>),
    Flush(oneshot::Sender<Result<()>>),
    CreateIndexes(oneshot::Sender<Result<()>>),
    Maintenance(oneshot::Sender<Result<()>>),
    Shutdown(oneshot::Sender<Result<()>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderBy {
    Similarity(Direction),
    CreatedUtc(Direction),
    Score(Direction),
    NumComments(Direction),
    Subscribers(Direction),
}

#[derive(Debug, Clone)]
pub struct Pagination {
    pub limit: usize,
    pub offset: usize,
    pub order_by: OrderBy,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            limit: 20,
            offset: 0,
            order_by: OrderBy::CreatedUtc(Direction::Desc),
        }
    }
}

/// Parameters for Reddit store search queries.
#[derive(Debug, Clone, Default)]
pub struct SearchParams {
    pub vector: Option<VectorQuery>,
    pub full_text: Option<String>,
    pub filter: String,
    pub pagination: Pagination,
}

/// Cloneable handle to the Reddit store.
///
/// Reads bypass the actor and open datasets directly (concurrent, MVCC-safe).
/// Writes are serialized through a background actor that batches and flushes.
#[derive(Clone)]
pub struct Store {
    items_uri: Arc<str>,
    path_key: String,
    tx: mpsc::UnboundedSender<Command>,
}

impl Store {
    /// Open or create dataset at `base_path/items` and spawn the background actor.
    ///
    /// Only one `Store` may be open per `base_path` at a time.
    /// Call `shutdown()` to release before reopening.
    pub async fn open(base_path: impl AsRef<Path>) -> Result<Self> {
        let base = base_path.as_ref();
        let items_uri: Arc<str> = base.join("items").to_string_lossy().into_owned().into();

        let path_key = base.to_string_lossy().into_owned();
        {
            let mut set = OPEN_PATHS.lock().unwrap();
            if !set.insert(path_key.clone()) {
                return Err(anyhow::anyhow!(
                    "Store already open for path: {}",
                    base.display()
                ));
            }
        }

        if let Err(e) = Self::ensure_dataset(&items_uri, Item::arrow_schema()).await {
            OPEN_PATHS.lock().unwrap().remove(&path_key);
            return Err(e);
        }

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(run_actor(Arc::clone(&items_uri), rx));

        Ok(Self {
            items_uri,
            path_key: path_key.clone(),
            tx,
        })
    }

    /// Queue items for buffered append. Returns immediately (fire-and-forget).
    pub fn append(&self, items: &[Item]) -> Result<()> {
        self.tx
            .send(Command::AppendItems(items.to_vec()))
            .map_err(|_| anyhow::anyhow!("actor closed"))
    }

    /// Force flush of buffer. Waits until written to disk.
    pub async fn flush(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Flush(tx))
            .map_err(|_| anyhow::anyhow!("actor closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("actor dropped reply"))?
    }

    /// Create indexes. Blocks until complete.
    pub async fn create_indexes(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::CreateIndexes(tx))
            .map_err(|_| anyhow::anyhow!("actor closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("actor dropped reply"))?
    }

    /// Run compaction + index optimization. Blocks until complete.
    pub async fn maintenance(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Maintenance(tx))
            .map_err(|_| anyhow::anyhow!("actor closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("actor dropped reply"))?
    }

    /// Shut down the background actor, flush pending buffers, and release the path lock.
    pub async fn shutdown(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Shutdown(tx))
            .map_err(|_| anyhow::anyhow!("actor closed"))?;
        let result = rx
            .await
            .map_err(|_| anyhow::anyhow!("actor dropped reply"))?;
        OPEN_PATHS.lock().unwrap().remove(&self.path_key);
        result
    }

    /// Search items by vector similarity, scalar filter, or both.
    /// Reads bypass the actor — opens dataset directly.
    pub async fn search(&self, params: &SearchParams) -> Result<SearchResult> {
        let dataset = Dataset::open(&self.items_uri).await?;

        let mut scanner = dataset.scan();
        let mut columns = vec![
            "name",
            "kind",
            "title",
            "content",
            "subreddit",
            "subreddit_id",
            "author",
            "score",
            "num_comments",
            "over_18",
            "permalink",
            "url",
            "post_id",
            "parent_id",
            "depth",
            "subscribers",
            "created_utc",
        ];

        scanner.filter(&params.filter)?;

        if let Some(vq) = &params.vector {
            columns.push("_distance");
            scanner.prefilter(true);
            scanner.distance_metric(lance_linalg::distance::DistanceType::Cosine);

            let query_arr = Float32Array::from(vq.embedding.to_vec());
            scanner
                .nearest("embedding", &query_arr, vq.top_n)
                .context("nearest search")?;
        }

        if let Some(fts) = &params.full_text {
            let query: FtsQuery = MatchQuery::new(fts.clone())
                .with_operator(Operator::And)
                .with_column(Some("search_text".into()))
                .into();
            scanner
                .full_text_search(FullTextSearchQuery::new_query(query))
                .context("full text search")?;
        }

        let total = scanner.count_rows().await.context("counting rows")?;

        match &params.pagination.order_by {
            OrderBy::Similarity(_) => { /* already sorted by nearest() */ }
            OrderBy::CreatedUtc(d) => {
                scanner.order_by(Some(vec![col_ordering("created_utc", *d)]))?;
            }
            OrderBy::Score(d) => {
                scanner.order_by(Some(vec![col_ordering("score", *d)]))?;
            }
            OrderBy::NumComments(d) => {
                scanner.order_by(Some(vec![col_ordering("num_comments", *d)]))?;
            }
            OrderBy::Subscribers(d) => {
                scanner.order_by(Some(vec![col_ordering("subscribers", *d)]))?;
            }
        }

        scanner.project(&columns)?;
        scanner
            .limit(
                Some(params.pagination.limit as i64),
                Some(params.pagination.offset as i64),
            )
            .context("setting limit/offset")?;

        let batches: Vec<RecordBatch> = scanner.try_into_stream().await?.try_collect().await?;
        let items: Vec<ScoredItem> = batches
            .iter()
            .map(Self::batch_to_scored_items)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();

        Ok(SearchResult { items, total })
    }

    /// Fetch a post and all its comments as a thread.
    pub async fn thread(&self, post_id: &str) -> Result<Thread> {
        let post_params = SearchParams {
            vector: None,
            full_text: None,
            filter: format!("name = '{}' AND kind = 'post'", post_id),
            pagination: Pagination {
                limit: 1,
                ..Default::default()
            },
        };
        let post_fut = self.search(&post_params);

        let comments_params = SearchParams {
            vector: None,
            full_text: None,
            filter: format!("post_id = '{}' AND kind = 'comment'", post_id),
            pagination: Pagination {
                limit: 10000,
                ..Default::default()
            },
        };
        let comments_fut = self.search(&comments_params);

        let (post_result, comments_result) = tokio::try_join!(post_fut, comments_fut)?;

        let post = post_result
            .items
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("post not found"))?
            .item;
        let comments = comments_result.items.into_iter().map(|s| s.item).collect();

        Ok(Thread { post, comments })
    }

    /// List index names.
    pub async fn indices(&self) -> Result<Vec<String>> {
        let dataset = Dataset::open(&self.items_uri).await?;
        let indices = dataset.load_indices().await.context("loading indices")?;
        Ok(indices.iter().map(|idx| idx.name.clone()).collect())
    }

    // -- helpers ------------------------------------------------------------

    async fn ensure_dataset(uri: &str, schema: Schema) -> Result<()> {
        if Dataset::open(uri).await.is_ok() {
            return Ok(());
        }
        let path = Path::new(uri);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating dir {}", parent.display()))?;
        }
        let schema = Arc::new(schema);
        let empty = RecordBatch::new_empty(schema.clone());
        let reader = RecordBatchIterator::new(vec![Ok(empty)], schema.clone());
        let params = WriteParams {
            mode: WriteMode::Create,
            ..Default::default()
        };
        Dataset::write(reader, uri, Some(params))
            .await
            .with_context(|| format!("creating dataset at {uri}"))?;
        Ok(())
    }

    fn batch_reader(
        batch: RecordBatch,
    ) -> impl arrow::record_batch::RecordBatchReader + Send + 'static {
        let schema = batch.schema();
        RecordBatchIterator::new(vec![Ok(batch)], schema)
    }

    fn batch_to_scored_items(batch: &RecordBatch) -> Result<Vec<ScoredItem>> {
        let items = Self::batch_to_items(batch)?;
        let distances = batch
            .column_by_name("_distance")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

        Ok(items
            .into_iter()
            .enumerate()
            .map(|(i, item)| ScoredItem {
                item,
                similarity: distances.map(|arr| 1.0 - arr.value(i) / 4.0),
            })
            .collect())
    }

    fn batch_to_items(batch: &RecordBatch) -> Result<Vec<Item>> {
        let names = as_string_array(batch.column_by_name("name").context("name column")?);
        let kinds = as_string_array(batch.column_by_name("kind").context("kind column")?);
        let titles = as_string_array(batch.column_by_name("title").context("title column")?);
        let contents = as_string_array(batch.column_by_name("content").context("content column")?);
        let subreddits = as_string_array(
            batch
                .column_by_name("subreddit")
                .context("subreddit column")?,
        );
        let subreddit_ids = as_string_array(
            batch
                .column_by_name("subreddit_id")
                .context("subreddit_id column")?,
        );
        let authors = as_string_array(batch.column_by_name("author").context("author column")?);
        let scores =
            as_primitive_array::<Int32Type>(batch.column_by_name("score").context("score column")?);
        let num_comments = as_primitive_array::<Int32Type>(
            batch
                .column_by_name("num_comments")
                .context("num_comments column")?,
        );
        let over_18s = as_boolean_array(batch.column_by_name("over_18").context("over_18 column")?);
        let permalinks = as_string_array(
            batch
                .column_by_name("permalink")
                .context("permalink column")?,
        );
        let urls = as_string_array(batch.column_by_name("url").context("url column")?);
        let post_ids = as_string_array(batch.column_by_name("post_id").context("post_id column")?);
        let parent_ids = as_string_array(
            batch
                .column_by_name("parent_id")
                .context("parent_id column")?,
        );
        let depths = as_primitive_array::<UInt32Type>(
            batch.column_by_name("depth").context("depth column")?,
        );
        let subscribers = as_primitive_array::<Int64Type>(
            batch
                .column_by_name("subscribers")
                .context("subscribers column")?,
        );
        let created_utcs = as_primitive_array::<TimestampSecondType>(
            batch
                .column_by_name("created_utc")
                .context("created_utc column")?,
        );

        Ok((0..batch.num_rows())
            .map(|i| {
                let kind = kinds.value(i);
                if kind == Kind::Post.as_str() {
                    Item::Post(Post {
                        name: names.value(i).to_string(),
                        title: titles.value(i).to_string(),
                        content: contents.value(i).to_string(),
                        subreddit: subreddits.value(i).to_string(),
                        subreddit_id: subreddit_ids.value(i).to_string(),
                        author: authors.value(i).to_string(),
                        score: scores.value(i),
                        num_comments: num_comments.value(i),
                        over_18: over_18s.value(i),
                        permalink: permalinks.value(i).to_string(),
                        url: urls.value(i).to_string(),
                        subscribers: subscribers.value(i),
                        embedding: vec![],
                        created_utc: DateTime::from_timestamp(created_utcs.value(i), 0).unwrap(),
                    })
                } else {
                    Item::Comment(Comment {
                        name: names.value(i).to_string(),
                        title: titles.value(i).to_string(),
                        content: contents.value(i).to_string(),
                        subreddit: subreddits.value(i).to_string(),
                        subreddit_id: subreddit_ids.value(i).to_string(),
                        author: authors.value(i).to_string(),
                        score: scores.value(i),
                        num_comments: num_comments.value(i),
                        over_18: over_18s.value(i),
                        permalink: permalinks.value(i).to_string(),
                        url: urls.value(i).to_string(),
                        post_id: post_ids.value(i).to_string(),
                        parent_id: parent_ids.value(i).to_string(),
                        depth: depths.value(i),
                        subscribers: subscribers.value(i),
                        embedding: vec![],
                        created_utc: DateTime::from_timestamp(created_utcs.value(i), 0).unwrap(),
                    })
                }
            })
            .collect())
    }
}

fn col_ordering(col: &str, dir: Direction) -> ColumnOrdering {
    match dir {
        Direction::Asc => ColumnOrdering::asc_nulls_last(col.to_string()),
        Direction::Desc => ColumnOrdering::desc_nulls_last(col.to_string()),
    }
}

// -- background actor -------------------------------------------------------

async fn run_actor(items_uri: Arc<str>, mut rx: mpsc::UnboundedReceiver<Command>) {
    let mut buf = Vec::new();
    let mut flush_tick = interval(Duration::from_secs(FLUSH_INTERVAL_SECS));

    loop {
        tokio::select! {
            Some(cmd) = rx.recv() => match cmd {
                Command::AppendItems(items) => {
                    buf.extend(items);
                    if buf.len() >= BATCH_SIZE
                        && let Err(e) = flush_items(&items_uri, &mut buf).await {
                            error!(error = %e, "flush items (size trigger) failed");
                        }
                }
                Command::Flush(reply) => {
                    let r = flush_items(&items_uri, &mut buf).await;
                    let _ = reply.send(r);
                }
                Command::CreateIndexes(reply) => {
                    let r = create_indexes(&items_uri).await;
                    let _ = reply.send(r);
                }
                Command::Maintenance(reply) => {
                    let r = async {
                        flush_items(&items_uri, &mut buf).await?;
                        run_maintenance(&items_uri).await
                    }.await;
                    let _ = reply.send(r);
                }
                Command::Shutdown(reply) => {
                    let r = flush_items(&items_uri, &mut buf).await;
                    let _ = reply.send(r);
                    break;
                }
            },
            _ = flush_tick.tick() => {
                if let Err(e) = flush_items(&items_uri, &mut buf).await {
                    error!(error = %e, "timer flush items failed");
                }
            }
        }
    }
}

fn items_to_record_batch(items: &[Item]) -> Result<RecordBatch> {
    crate::record_batch! {
        schema = Item::arrow_schema(),
        items = items,
        each = item,
        builders = [
            name: StringBuilder::new() => name.append_value(item.name()),
            kind: StringBuilder::new() => kind.append_value(item.kind().as_str()),
            title: StringBuilder::new() => title.append_value(item.title()),
            content: StringBuilder::new() => content.append_value(item.content()),
            search_text: StringBuilder::new() => search_text.append_value(format!("{} {}", item.title(), item.content())),
            subreddit: StringBuilder::new() => subreddit.append_value(item.subreddit()),
            subreddit_id: StringBuilder::new() => subreddit_id.append_value(item.subreddit_id()),
            author: StringBuilder::new() => author.append_value(item.author()),
            score: Int32Builder::new() => score.append_value(item.score()),
            num_comments: Int32Builder::new() => num_comments.append_value(item.num_comments()),
            over_18: BooleanBuilder::new() => over_18.append_value(item.over_18()),
            permalink: StringBuilder::new() => permalink.append_value(item.permalink()),
            url: StringBuilder::new() => url.append_value(item.url()),
            post_id: StringBuilder::new() => match item.post_id() {
                Some(v) => post_id.append_value(v),
                None => post_id.append_null(),
            },
            parent_id: StringBuilder::new() => match item.parent_id() {
                Some(v) => parent_id.append_value(v),
                None => parent_id.append_null(),
            },
            depth: UInt32Builder::new() => match item.depth() {
                Some(v) => depth.append_value(v),
                None => depth.append_null(),
            },
            subscribers: Int64Builder::new() => subscribers.append_value(item.subscribers()),
            embedding: FixedSizeListBuilder::new(Float32Builder::new(), EMBEDDING_DIM) => {
                embedding.values().append_slice(item.embedding());
                embedding.append(true);
            },
            created_utc: TimestampSecondBuilder::new() => {
                created_utc.append_value(item.created_utc().timestamp())
            },
        ]
    }
}

async fn flush_items(uri: &str, buf: &mut Vec<Item>) -> Result<()> {
    if buf.is_empty() {
        return Ok(());
    }
    let batch = items_to_record_batch(buf)?;
    let params = WriteParams {
        mode: WriteMode::Append,
        ..Default::default()
    };
    Dataset::write(Store::batch_reader(batch), uri, Some(params))
        .await
        .context("appending items")?;
    info!(count = buf.len(), "flushed items");
    buf.clear();
    Ok(())
}

async fn create_indexes(uri: &str) -> Result<()> {
    let mut dataset = Dataset::open(uri).await?;
    let row_count = dataset.count_rows(None).await.context("counting rows")?;
    let indices = dataset.load_indices().await.context("loading indices")?;
    let scalar_params = ScalarIndexParams::default();

    if row_count >= 256 && !indices.iter().any(|idx| idx.name == "embedding_idx") {
        let vector_params = lance::index::vector::VectorIndexParams::with_ivf_hnsw_pq_params(
            lance_linalg::distance::DistanceType::Cosine,
            IvfBuildParams::default(),
            HnswBuildParams::default(),
            PQBuildParams::default(),
        );
        dataset
            .create_index_builder(&["embedding"], IndexType::IvfHnswPq, &vector_params)
            .name("embedding_idx".into())
            .replace(true)
            .await
            .context("creating vector index on embedding")?;
    }

    if !indices.iter().any(|idx| idx.name == "post_id_idx") {
        dataset
            .create_index_builder(&["post_id"], IndexType::BTree, &scalar_params)
            .name("post_id_idx".into())
            .replace(true)
            .await
            .context("creating BTree index on post_id")?;
    }

    if !indices.iter().any(|idx| idx.name == "created_utc_idx") {
        dataset
            .create_index_builder(&["created_utc"], IndexType::BTree, &scalar_params)
            .name("created_utc_idx".into())
            .replace(true)
            .await
            .context("creating BTree index on created_utc")?;
    }

    if !indices.iter().any(|idx| idx.name == "subreddit_idx") {
        dataset
            .create_index_builder(&["subreddit"], IndexType::Bitmap, &scalar_params)
            .name("subreddit_idx".into())
            .replace(true)
            .await
            .context("creating Bitmap index on subreddit")?;
    }

    if !indices.iter().any(|idx| idx.name == "subscribers_idx") {
        dataset
            .create_index_builder(&["subscribers"], IndexType::BTree, &scalar_params)
            .name("subscribers_idx".into())
            .replace(true)
            .await
            .context("creating BTree index on subscribers")?;
    }

    let inverted_params = InvertedIndexParams::default();
    if !indices.iter().any(|idx| idx.name == "depth_idx") {
        dataset
            .create_index_builder(&["depth"], IndexType::BTree, &scalar_params)
            .name("depth_idx".into())
            .replace(true)
            .await
            .context("creating BTree index on depth")?;
    }

    if !indices.iter().any(|idx| idx.name == "search_text_idx") {
        dataset
            .create_index_builder(&["search_text"], IndexType::Inverted, &inverted_params)
            .name("search_text_idx".into())
            .replace(true)
            .await
            .context("creating inverted index on search_text")?;
    }

    Ok(())
}

async fn run_maintenance(uri: &str) -> Result<()> {
    let mut dataset = Dataset::open(uri).await?;
    compact_files(&mut dataset, Default::default(), None)
        .await
        .context("compacting dataset")?;

    let opts = lance_index::optimize::OptimizeOptions::default();
    dataset
        .optimize_indices(&opts)
        .await
        .context("optimizing indices")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::types::{Direction, EMBEDDING_DIM, VectorQuery};
    use arrow_array::StringArray;
    use chrono::{DateTime, Utc};
    use tempfile::TempDir;

    fn test_date_filter() -> String {
        "created_utc >= timestamp '2024-04-01 00:00:00' AND created_utc < timestamp '2024-06-01 00:00:00'".to_string()
    }

    #[test]
    fn item_schema_fields() {
        let schema = Item::arrow_schema();
        assert_eq!(schema.fields().len(), 19);
        assert!(schema.field_with_name("name").is_ok());
        assert!(schema.field_with_name("kind").is_ok());
        assert!(schema.field_with_name("title").is_ok());
        assert!(schema.field_with_name("embedding").is_ok());
    }

    #[test]
    fn post_to_record_batch() {
        let items = vec![fake_item_post("t3_test1", 100)];
        let batch = items_to_record_batch(&items).unwrap();
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 19);

        let names = batch.column_by_name("name").unwrap();
        let arr = names.as_any().downcast_ref::<StringArray>().unwrap();
        assert_eq!(arr.value(0), "t3_test1");

        let kinds = batch.column_by_name("kind").unwrap();
        let arr = kinds.as_any().downcast_ref::<StringArray>().unwrap();
        assert_eq!(arr.value(0), "post");

        let titles = batch.column_by_name("title").unwrap();
        let arr = titles.as_any().downcast_ref::<StringArray>().unwrap();
        assert_eq!(arr.value(0), "Post t3_test1");
    }

    #[test]
    fn comment_to_record_batch() {
        let items = vec![fake_item_comment("t1_test1", "t3_p1", 50)];
        let batch = items_to_record_batch(&items).unwrap();
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 19);

        let post_ids = batch.column_by_name("post_id").unwrap();
        let arr = post_ids.as_any().downcast_ref::<StringArray>().unwrap();
        assert_eq!(arr.value(0), "t3_p1");

        let kinds = batch.column_by_name("kind").unwrap();
        let arr = kinds.as_any().downcast_ref::<StringArray>().unwrap();
        assert_eq!(arr.value(0), "comment");

        let titles = batch.column_by_name("title").unwrap();
        let arr = titles.as_any().downcast_ref::<StringArray>().unwrap();
        assert_eq!(arr.value(0), "Post title");
    }

    #[test]
    fn item_json_has_kind_and_camel_case_fields() {
        let post = serde_json::to_value(fake_item_post("t3_test", 100)).unwrap();
        assert_eq!(post.get("kind").unwrap(), "post");
        assert!(post.get("numComments").is_some());
        assert!(post.get("num_comments").is_none());
        assert!(post.get("embedding").is_none());
        assert_eq!(post.get("score").unwrap(), 100);

        let comment = serde_json::to_value(fake_item_comment("t1_test", "t3_p", 50)).unwrap();
        assert_eq!(comment.get("kind").unwrap(), "comment");
        assert!(comment.get("postId").is_some());
        assert!(comment.get("post_id").is_none());
        assert!(comment.get("embedding").is_none());
        assert_eq!(comment.get("depth").unwrap(), 0);
    }

    fn fake_item_post(name: &str, score: i32) -> Item {
        Item::Post(Post {
            name: name.into(),
            title: format!("Post {}", name),
            content: "body".into(),
            subreddit: "test".into(),
            subreddit_id: "t5_test".into(),
            author: "a".into(),
            score,
            num_comments: 0,
            over_18: false,
            permalink: "/r/test".into(),
            url: "".into(),
            subscribers: 1234,
            embedding: vec![0.1f32; EMBEDDING_DIM as usize],
            created_utc: DateTime::from_timestamp(1714584000, 0).unwrap(),
        })
    }

    fn fake_item_comment(name: &str, post_id: &str, score: i32) -> Item {
        Item::Comment(Comment {
            name: name.into(),
            title: "Post title".into(),
            content: format!("comment {}", name),
            subreddit: "test".into(),
            subreddit_id: "t5_test".into(),
            author: "c".into(),
            score,
            num_comments: 5,
            over_18: false,
            permalink: "/r/test/comments/test1/t1_abc".into(),
            url: "".into(),
            post_id: post_id.into(),
            parent_id: post_id.into(),
            depth: 0,
            subscribers: 1234,
            embedding: vec![0.3f32; EMBEDDING_DIM as usize],
            created_utc: DateTime::from_timestamp(1714584100, 0).unwrap(),
        })
    }

    #[tokio::test]
    async fn open_create_and_append_posts() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let posts = vec![fake_item_post("t3_a", 100), fake_item_post("t3_b", 200)];
        store.append(&posts).unwrap();
        store.flush().await.unwrap();

        // Clone handle — same underlying actor, no need to reopen
        let store2 = store.clone();
        let query = vec![0.2f32; EMBEDDING_DIM as usize];
        let items: Vec<ScoredItem> = store2
            .search(&SearchParams {
                full_text: None,
                vector: Some(VectorQuery {
                    embedding: query,
                    top_n: 10,
                }),
                filter: format!("{} AND kind = 'post'", test_date_filter()),
                pagination: Pagination {
                    order_by: OrderBy::Similarity(Direction::Desc),
                    ..Default::default()
                },
            })
            .await
            .unwrap()
            .items;
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|s| matches!(s.item, Item::Post(_))));
    }

    #[tokio::test]
    async fn append_comments_and_query() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let posts = vec![fake_item_post("t3_p1", 100), fake_item_post("t3_p2", 200)];
        let comments = vec![
            fake_item_comment("t1_a", "t3_p1", 10),
            fake_item_comment("t1_b", "t3_p1", 20),
            fake_item_comment("t1_c", "t3_p2", 30),
        ];
        store.append(&posts).unwrap();
        store.append(&comments).unwrap();
        store.flush().await.unwrap();

        let p1 = store.thread("t3_p1").await.unwrap();
        assert_eq!(p1.post.name(), "t3_p1");
        assert_eq!(p1.comments.len(), 2);
        assert!(p1.comments.iter().any(|c| c.name() == "t1_a"));
        assert!(p1.comments.iter().any(|c| c.name() == "t1_b"));

        let p2 = store.thread("t3_p2").await.unwrap();
        assert_eq!(p2.post.name(), "t3_p2");
        assert_eq!(p2.comments.len(), 1);
    }

    #[tokio::test]
    async fn post_datetime_round_trip() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let original = fake_item_post("t3_dt", 42);
        store.append(&[original.clone()]).unwrap();
        store.flush().await.unwrap();

        let query = vec![0.1f32; EMBEDDING_DIM as usize];
        let scored: Vec<ScoredItem> = store
            .search(&SearchParams {
                full_text: None,
                vector: Some(VectorQuery {
                    embedding: query,
                    top_n: 10,
                }),
                filter: format!("{} AND kind = 'post'", test_date_filter()),
                pagination: Pagination {
                    order_by: OrderBy::Similarity(Direction::Desc),
                    ..Default::default()
                },
            })
            .await
            .unwrap()
            .items;
        assert_eq!(scored.len(), 1);
        let Item::Post(post) = &scored[0].item else {
            panic!("expected post")
        };
        assert_eq!(
            post.created_utc,
            DateTime::from_timestamp(1714584000, 0).unwrap()
        );
    }

    #[tokio::test]
    async fn comment_datetime_round_trip() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let post = fake_item_post("t3_p1", 100);
        let original = fake_item_comment("t1_dt", "t3_p1", 42);
        store.append(&[post]).unwrap();
        store.append(&[original.clone()]).unwrap();
        store.flush().await.unwrap();

        let thread = store.thread("t3_p1").await.unwrap();
        assert_eq!(thread.comments.len(), 1);
        let Item::Comment(ref comment) = thread.comments[0] else {
            panic!("expected comment")
        };
        assert_eq!(
            comment.created_utc,
            DateTime::from_timestamp(1714584100, 0).unwrap()
        );
    }

    #[tokio::test]
    async fn shutdown_then_reopen() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();
        store.shutdown().await.unwrap();

        let store2 = Store::open(tmp.path()).await.unwrap();
        store2.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn double_open_fails() {
        let tmp = TempDir::new().unwrap();
        let _store = Store::open(tmp.path()).await.unwrap();
        let result = Store::open(tmp.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn vector_search_orders_by_similarity() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let mut emb_a = vec![0.0f32; EMBEDDING_DIM as usize];
        emb_a[0] = 1.0;
        let mut emb_b = vec![0.0f32; EMBEDDING_DIM as usize];
        emb_b[1] = 1.0;

        let posts = vec![
            fake_item_post_with_embedding("t3_close", 100, emb_a),
            fake_item_post_with_embedding("t3_far", 100, emb_b),
        ];
        store.append(&posts).unwrap();
        store.flush().await.unwrap();

        let mut query = vec![0.0f32; EMBEDDING_DIM as usize];
        query[0] = 1.0;

        let scored: Vec<ScoredItem> = store
            .search(&SearchParams {
                full_text: None,
                vector: Some(VectorQuery {
                    embedding: query.clone(),
                    top_n: 2,
                }),
                filter: format!("{} AND kind = 'post'", test_date_filter()),
                pagination: Pagination {
                    order_by: OrderBy::Similarity(Direction::Desc),
                    ..Default::default()
                },
            })
            .await
            .unwrap()
            .items;
        assert_eq!(scored.len(), 2);
        assert_eq!(scored[0].item.name(), "t3_close");
        assert_eq!(scored[1].item.name(), "t3_far");
        assert!(
            scored[0].similarity.unwrap() > scored[1].similarity.unwrap(),
            "first result should have higher similarity"
        );
    }

    #[tokio::test]
    async fn vector_plus_filter_excludes_low_score() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let mut emb_close = vec![0.0f32; EMBEDDING_DIM as usize];
        emb_close[0] = 1.0;
        let mut emb_far = vec![0.0f32; EMBEDDING_DIM as usize];
        emb_far[0] = 0.5;
        emb_far[1] = 0.5;

        let posts = vec![
            fake_item_post_with_embedding("t3_low_score", 50, emb_close),
            fake_item_post_with_embedding("t3_high_score", 200, emb_far),
        ];
        store.append(&posts).unwrap();
        store.flush().await.unwrap();

        let mut query = vec![0.0f32; EMBEDDING_DIM as usize];
        query[0] = 1.0;

        let filtered: Vec<ScoredItem> = store
            .search(&SearchParams {
                full_text: None,
                vector: Some(VectorQuery {
                    embedding: query.clone(),
                    top_n: 10,
                }),
                filter: format!("{} AND score > 100 AND kind = 'post'", test_date_filter()),
                pagination: Pagination {
                    order_by: OrderBy::Similarity(Direction::Desc),
                    ..Default::default()
                },
            })
            .await
            .unwrap()
            .items;

        assert_eq!(filtered.len(), 1, "only high score post should match");
        assert_eq!(filtered[0].item.name(), "t3_high_score");
    }

    #[tokio::test]
    async fn filter_returns_empty_when_nothing_matches() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let posts = vec![fake_item_post_with_embedding(
            "t3_a",
            50,
            vec![0.1f32; EMBEDDING_DIM as usize],
        )];
        store.append(&posts).unwrap();
        store.flush().await.unwrap();

        let query = vec![0.1f32; EMBEDDING_DIM as usize];
        let result: Vec<ScoredItem> = store
            .search(&SearchParams {
                full_text: None,
                vector: Some(VectorQuery {
                    embedding: query.clone(),
                    top_n: 10,
                }),
                filter: format!("{} AND score > 1000 AND kind = 'post'", test_date_filter()),
                pagination: Pagination {
                    order_by: OrderBy::Similarity(Direction::Desc),
                    ..Default::default()
                },
            })
            .await
            .unwrap()
            .items;

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn create_indexes_and_search() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let mut posts = Vec::new();
        for i in 0..300 {
            let mut emb = vec![0.0f32; EMBEDDING_DIM as usize];
            emb[i % EMBEDDING_DIM as usize] = 1.0;
            posts.push(fake_item_post_with_embedding(
                &format!("t3_{}", i),
                i as i32,
                emb,
            ));
        }
        store.append(&posts).unwrap();
        store.flush().await.unwrap();

        store.create_indexes().await.unwrap();

        let indices = store.indices().await.unwrap();
        assert!(!indices.is_empty(), "no indices found after creation");
        assert!(
            indices.iter().any(|idx| idx == "embedding_idx"),
            "vector index on embedding missing"
        );

        let mut query = vec![0.0f32; EMBEDDING_DIM as usize];
        query[0] = 1.0;
        let scored: Vec<ScoredItem> = store
            .search(&SearchParams {
                full_text: None,
                vector: Some(VectorQuery {
                    embedding: query.clone(),
                    top_n: 5,
                }),
                filter: format!("{} AND kind = 'post'", test_date_filter()),
                pagination: Pagination {
                    order_by: OrderBy::Similarity(Direction::Desc),
                    ..Default::default()
                },
            })
            .await
            .unwrap()
            .items;

        assert!(
            !scored.is_empty(),
            "search with index should return results"
        );
        assert_eq!(scored[0].item.name(), "t3_0");
        for i in 1..scored.len() {
            assert!(
                scored[i - 1].similarity.unwrap() >= scored[i].similarity.unwrap(),
                "results should be ordered by similarity"
            );
        }

        store.create_indexes().await.unwrap();
    }

    #[tokio::test]
    async fn cosine_similarity_range() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let mut emb_a = vec![0.0f32; EMBEDDING_DIM as usize];
        emb_a[0] = 1.0;
        let mut emb_b = vec![0.0f32; EMBEDDING_DIM as usize];
        emb_b[1] = 1.0;
        let mut emb_c = vec![0.0f32; EMBEDDING_DIM as usize];
        emb_c[0] = -1.0;

        store
            .append(&[
                fake_item_post_with_embedding("t3_a", 100, emb_a),
                fake_item_post_with_embedding("t3_b", 100, emb_b),
                fake_item_post_with_embedding("t3_c", 100, emb_c),
            ])
            .unwrap();
        store.flush().await.unwrap();

        let mut query = vec![0.0f32; EMBEDDING_DIM as usize];
        query[0] = 1.0;

        let scored: Vec<ScoredItem> = store
            .search(&SearchParams {
                full_text: None,
                vector: Some(VectorQuery {
                    embedding: query.clone(),
                    top_n: 10,
                }),
                filter: format!("{} AND kind = 'post'", test_date_filter()),
                pagination: Pagination {
                    order_by: OrderBy::Similarity(Direction::Desc),
                    ..Default::default()
                },
            })
            .await
            .unwrap()
            .items;

        let sim_a = scored
            .iter()
            .find(|s| s.item.name() == "t3_a")
            .unwrap()
            .similarity
            .unwrap();
        let sim_b = scored
            .iter()
            .find(|s| s.item.name() == "t3_b")
            .unwrap()
            .similarity
            .unwrap();
        let sim_c = scored
            .iter()
            .find(|s| s.item.name() == "t3_c")
            .unwrap()
            .similarity
            .unwrap();

        assert!(
            sim_a > sim_b && sim_b > sim_c,
            "expected sim_a > sim_b > sim_c, got {} > {} > {}",
            sim_a,
            sim_b,
            sim_c
        );
        assert!(
            (sim_a - 1.0).abs() < 1e-5,
            "identical vector should have similarity ≈ 1.0, got {}",
            sim_a
        );
        assert!(
            (sim_b - 0.5).abs() < 1e-5,
            "orthogonal vector should have similarity ≈ 0.5, got {}",
            sim_b
        );
        assert!(
            sim_c < 1e-5,
            "opposite vector should have similarity ≈ 0.0, got {}",
            sim_c
        );
    }

    #[tokio::test]
    async fn create_post_id_index() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let comments = vec![
            fake_item_comment("t1_a", "t3_p1", 10),
            fake_item_comment("t1_b", "t3_p1", 20),
            fake_item_comment("t1_c", "t3_p2", 30),
        ];
        store.append(&comments).unwrap();
        store.flush().await.unwrap();

        store.create_indexes().await.unwrap();

        let indices = store.indices().await.unwrap();
        assert!(
            indices.iter().any(|idx| idx == "post_id_idx"),
            "post_id index missing, got {:?}",
            indices
        );

        store.create_indexes().await.unwrap();
    }

    #[tokio::test]
    async fn filter_timestamp_literal() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let old = fake_item_post("t3_old", 100);
        let mut new_post = match fake_item_post("t3_new", 200) {
            Item::Post(p) => p,
            _ => panic!("expected post"),
        };
        new_post.created_utc = Utc::now();
        let new = Item::Post(new_post);

        store.append(&[old, new]).unwrap();
        store.flush().await.unwrap();

        // Date range covers both posts, plus additional filter
        let filter = "created_utc >= timestamp '2024-04-01 00:00:00' AND created_utc > timestamp '2024-05-01 00:00:00'";
        let result: Vec<ScoredItem> = store
            .search(&SearchParams {
                full_text: None,
                vector: None,
                filter: filter.to_string(),
                pagination: Pagination {
                    limit: 10,
                    ..Default::default()
                },
            })
            .await
            .unwrap()
            .items;
        // Should return at least the "new" post
        assert!(
            result.iter().any(|r| r.item.name() == "t3_new"),
            "timestamp literal filter should match recent post"
        );
    }

    #[tokio::test]
    async fn comment_vector_search_orders_by_similarity() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let mut emb_a = vec![0.0f32; EMBEDDING_DIM as usize];
        emb_a[0] = 1.0;
        let mut emb_b = vec![0.0f32; EMBEDDING_DIM as usize];
        emb_b[1] = 1.0;

        let comments = vec![
            fake_item_comment_with_embedding("t1_close", "t3_p", 10, emb_a),
            fake_item_comment_with_embedding("t1_far", "t3_p", 20, emb_b),
        ];
        store.append(&comments).unwrap();
        store.flush().await.unwrap();

        let mut query = vec![0.0f32; EMBEDDING_DIM as usize];
        query[0] = 1.0;

        let scored: Vec<ScoredItem> = store
            .search(&SearchParams {
                full_text: None,
                vector: Some(VectorQuery {
                    embedding: query,
                    top_n: 2,
                }),
                filter: format!("{} AND kind = 'comment'", test_date_filter()),
                pagination: Pagination {
                    order_by: OrderBy::Similarity(Direction::Desc),
                    ..Default::default()
                },
            })
            .await
            .unwrap()
            .items;
        assert_eq!(scored.len(), 2);
        assert_eq!(scored[0].item.name(), "t1_close");
        assert_eq!(scored[1].item.name(), "t1_far");
        assert!(
            scored[0].similarity.unwrap() > scored[1].similarity.unwrap(),
            "first result should have higher similarity"
        );
    }

    #[tokio::test]
    async fn comment_vector_plus_filter() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let mut emb_close = vec![0.0f32; EMBEDDING_DIM as usize];
        emb_close[0] = 1.0;
        let mut emb_far = vec![0.0f32; EMBEDDING_DIM as usize];
        emb_far[0] = 0.5;
        emb_far[1] = 0.5;

        let comments = vec![
            fake_item_comment_with_embedding("t1_low_score", "t3_p", 5, emb_close),
            fake_item_comment_with_embedding("t1_high_score", "t3_p", 50, emb_far),
        ];
        store.append(&comments).unwrap();
        store.flush().await.unwrap();

        let mut query = vec![0.0f32; EMBEDDING_DIM as usize];
        query[0] = 1.0;

        let filtered: Vec<ScoredItem> = store
            .search(&SearchParams {
                full_text: None,
                vector: Some(VectorQuery {
                    embedding: query,
                    top_n: 10,
                }),
                filter: format!("{} AND score > 10 AND kind = 'comment'", test_date_filter()),
                pagination: Pagination {
                    order_by: OrderBy::Similarity(Direction::Desc),
                    ..Default::default()
                },
            })
            .await
            .unwrap()
            .items;

        assert_eq!(filtered.len(), 1, "only high score comment should match");
        assert_eq!(filtered[0].item.name(), "t1_high_score");
    }

    #[tokio::test]
    async fn create_comment_indexes_and_search() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let mut comments = Vec::new();
        for i in 0..300 {
            let mut emb = vec![0.0f32; EMBEDDING_DIM as usize];
            emb[i % EMBEDDING_DIM as usize] = 1.0;
            comments.push(fake_item_comment_with_embedding(
                &format!("t1_{}", i),
                "t3_p",
                i as i32,
                emb,
            ));
        }
        store.append(&comments).unwrap();
        store.flush().await.unwrap();

        store.create_indexes().await.unwrap();

        let indices = store.indices().await.unwrap();
        assert!(!indices.is_empty(), "no indices found after creation");
        assert!(
            indices.iter().any(|idx| idx == "embedding_idx"),
            "vector index on embedding missing, got {:?}",
            indices
        );
        assert!(
            indices.iter().any(|idx| idx == "created_utc_idx"),
            "scalar index on created_utc missing, got {:?}",
            indices
        );
        assert!(
            indices.iter().any(|idx| idx == "subreddit_idx"),
            "scalar index on subreddit missing, got {:?}",
            indices
        );

        let mut query = vec![0.0f32; EMBEDDING_DIM as usize];
        query[0] = 1.0;
        let scored: Vec<ScoredItem> = store
            .search(&SearchParams {
                full_text: None,
                vector: Some(VectorQuery {
                    embedding: query,
                    top_n: 5,
                }),
                filter: format!("{} AND kind = 'comment'", test_date_filter()),
                pagination: Pagination {
                    order_by: OrderBy::Similarity(Direction::Desc),
                    ..Default::default()
                },
            })
            .await
            .unwrap()
            .items;

        assert!(
            !scored.is_empty(),
            "comment search with index should return results"
        );
        assert_eq!(scored[0].item.name(), "t1_0");
        for i in 1..scored.len() {
            assert!(
                scored[i - 1].similarity.unwrap() >= scored[i].similarity.unwrap(),
                "comment results should be ordered by similarity"
            );
        }

        store.create_indexes().await.unwrap();
    }

    #[tokio::test]
    async fn maintenance_runs_without_error() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let posts = vec![fake_item_post("t3_m", 100)];
        store.append(&posts).unwrap();
        store.flush().await.unwrap();

        store.maintenance().await.unwrap();
    }

    fn fake_item_post_with_embedding(name: &str, score: i32, embedding: Vec<f32>) -> Item {
        Item::Post(Post {
            name: name.into(),
            title: format!("Post {}", name),
            content: "body".into(),
            subreddit: "test".into(),
            subreddit_id: "t5_test".into(),
            author: "a".into(),
            score,
            num_comments: 0,
            over_18: false,
            permalink: "/r/test".into(),
            url: "".into(),
            subscribers: 1234,
            embedding,
            created_utc: DateTime::from_timestamp(1714584000, 0).unwrap(),
        })
    }

    fn fake_item_comment_with_embedding(
        name: &str,
        post_id: &str,
        score: i32,
        embedding: Vec<f32>,
    ) -> Item {
        Item::Comment(Comment {
            name: name.into(),
            title: "Post title".into(),
            content: format!("comment {}", name),
            subreddit: "test".into(),
            subreddit_id: "t5_test".into(),
            author: "c".into(),
            score,
            num_comments: 5,
            over_18: false,
            permalink: "/r/test/comments/test1/t1_abc".into(),
            url: "".into(),
            post_id: post_id.into(),
            parent_id: post_id.into(),
            depth: 0,
            subscribers: 1234,
            embedding,
            created_utc: DateTime::from_timestamp(1714584100, 0).unwrap(),
        })
    }

    #[tokio::test]
    async fn full_text_search_matches_title_and_content() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let post_a = Item::Post(Post {
            name: "t3_a".into(),
            title: "bitcoin mining guide".into(),
            content: "how to mine bitcoin".into(),
            subreddit: "test".into(),
            subreddit_id: "t5_test".into(),
            author: "a".into(),
            score: 100,
            num_comments: 0,
            over_18: false,
            permalink: "/r/test".into(),
            url: "".into(),
            subscribers: 1234,
            embedding: vec![0.1f32; EMBEDDING_DIM as usize],
            created_utc: DateTime::from_timestamp(1714584000, 0).unwrap(),
        });

        let post_b = Item::Post(Post {
            name: "t3_b".into(),
            title: "ethereum staking".into(),
            content: "staking eth is easy".into(),
            subreddit: "test".into(),
            subreddit_id: "t5_test".into(),
            author: "a".into(),
            score: 200,
            num_comments: 0,
            over_18: false,
            permalink: "/r/test".into(),
            url: "".into(),
            subscribers: 1234,
            embedding: vec![0.1f32; EMBEDDING_DIM as usize],
            created_utc: DateTime::from_timestamp(1714584000, 0).unwrap(),
        });

        let post_c = Item::Post(Post {
            name: "t3_c".into(),
            title: "bitcoin price analysis".into(),
            content: "market trends for btc".into(),
            subreddit: "test".into(),
            subreddit_id: "t5_test".into(),
            author: "a".into(),
            score: 300,
            num_comments: 0,
            over_18: false,
            permalink: "/r/test".into(),
            url: "".into(),
            subscribers: 1234,
            embedding: vec![0.1f32; EMBEDDING_DIM as usize],
            created_utc: DateTime::from_timestamp(1714584000, 0).unwrap(),
        });

        store.append(&[post_a, post_b, post_c]).unwrap();
        store.flush().await.unwrap();
        store.create_indexes().await.unwrap();

        // Search for "bitcoin" — should match posts a and c
        let results: Vec<ScoredItem> = store
            .search(&SearchParams {
                vector: None,
                full_text: Some("bitcoin".to_string()),
                filter: test_date_filter(),
                pagination: Pagination::default(),
            })
            .await
            .unwrap()
            .items;

        assert_eq!(results.len(), 2, "expected 2 posts matching 'bitcoin'");
        assert!(results.iter().any(|r| r.item.name() == "t3_a"));
        assert!(results.iter().any(|r| r.item.name() == "t3_c"));
        // Search for "bitcoin btc" (AND semantics) — should match only post c
        let results_and: Vec<ScoredItem> = store
            .search(&SearchParams {
                vector: None,
                full_text: Some("bitcoin btc".to_string()),
                filter: test_date_filter(),
                pagination: Pagination::default(),
            })
            .await
            .unwrap()
            .items;

        assert_eq!(
            results_and.len(),
            1,
            "expected 1 post matching 'bitcoin btc'"
        );
        assert_eq!(results_and[0].item.name(), "t3_c");
    }

    #[tokio::test]
    async fn pagination_total_counts_all_matching_rows() {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).await.unwrap();

        let posts: Vec<Item> = (0..5)
            .map(|i| fake_item_post(&format!("t3_{}", i), i as i32))
            .collect();
        store.append(&posts).unwrap();
        store.flush().await.unwrap();

        let result = store
            .search(&SearchParams {
                vector: None,
                full_text: None,
                filter: "kind = 'post'".into(),
                pagination: Pagination {
                    limit: 2,
                    offset: 0,
                    order_by: OrderBy::CreatedUtc(Direction::Desc),
                },
            })
            .await
            .unwrap();

        assert_eq!(result.items.len(), 2, "page should return 2 items");
        assert_eq!(result.total, 5, "total should count all 5 matching rows");
    }
}
