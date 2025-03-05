use meili_snap::snapshot;
use meilisearch::Opt;
use once_cell::sync::Lazy;
use tempfile::TempDir;

use crate::common::{default_settings, Server, Value, NESTED_DOCUMENTS};
use crate::json;

static DOCUMENTS: Lazy<Value> = Lazy::new(|| {
    json!([
        {
            "title": "Shazam!",
            "genres": ["Action", "Adventure"],
            "id": "287947",
        },
        {
            "title": "Captain Marvel",
            "genres": ["Action", "Adventure"],
            "id": "299537",
        },
        {
            "title": "Escape Room",
            "genres": ["Horror", "Thriller", "Multiple Words"],
            "id": "522681",
        },
        {
            "title": "How to Train Your Dragon: The Hidden World",
            "genres": ["Action", "Comedy"],
            "id": "166428",
        },
        {
            "title": "Gläss",
            "genres": ["Thriller"],
            "id": "450465",
        }
    ])
});

async fn test_settings_documents_indexing_swapping_and_facet_search(
    documents: &Value,
    settings: &Value,
    query: &Value,
    test: impl Fn(Value, actix_http::StatusCode) + std::panic::UnwindSafe + Clone,
) {
    let temp = TempDir::new().unwrap();
    let server = Server::new_with_options(Opt { ..default_settings(temp.path()) }).await.unwrap();

    eprintln!("Documents -> Settings -> test");
    let index = server.index("test");

    let (task, code) = index.add_documents(documents.clone(), None).await;
    assert_eq!(code, 202, "{}", task);
    let response = index.wait_task(task.uid()).await;
    assert!(response.is_success(), "{:?}", response);

    let (task, code) = index.update_settings(settings.clone()).await;
    assert_eq!(code, 202, "{}", task);
    let response = index.wait_task(task.uid()).await;
    assert!(response.is_success(), "{:?}", response);

    let (response, code) = index.facet_search(query.clone()).await;
    insta::allow_duplicates! {
        test(response, code);
    }

    let (task, code) = server.delete_index("test").await;
    assert_eq!(code, 202, "{}", task);
    let response = server.wait_task(task.uid()).await;
    assert!(response.is_success(), "{:?}", response);

    eprintln!("Settings -> Documents -> test");
    let index = server.index("test");

    let (task, code) = index.update_settings(settings.clone()).await;
    assert_eq!(code, 202, "{}", task);
    let response = index.wait_task(task.uid()).await;
    assert!(response.is_success(), "{:?}", response);

    let (task, code) = index.add_documents(documents.clone(), None).await;
    assert_eq!(code, 202, "{}", task);
    let response = index.wait_task(task.uid()).await;
    assert!(response.is_success(), "{:?}", response);

    let (response, code) = index.facet_search(query.clone()).await;
    insta::allow_duplicates! {
        test(response, code);
    }

    let (task, code) = server.delete_index("test").await;
    assert_eq!(code, 202, "{}", task);
    let response = server.wait_task(task.uid()).await;
    assert!(response.is_success(), "{:?}", response);
}

#[actix_rt::test]
async fn simple_facet_search() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = DOCUMENTS.clone();
    index.update_settings_filterable_attributes(json!(["genres"])).await;
    let (task, _status_code) = index.add_documents(documents, None).await;
    index.wait_task(task.uid()).await.succeeded();

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "a"})).await;

    assert_eq!(code, 200, "{}", response);
    assert_eq!(dbg!(response)["facetHits"].as_array().unwrap().len(), 2);

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "adventure"})).await;

    assert_eq!(code, 200, "{}", response);
    assert_eq!(response["facetHits"].as_array().unwrap().len(), 1);
}

#[actix_rt::test]
async fn simple_facet_search_on_movies() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = json!([
      {
        "id": 1,
        "title": "Carol",
        "genres": [
          "Romance",
          "Drama"
        ],
        "color": [
          "red"
        ],
        "platforms": [
          "MacOS",
          "Linux",
          "Windows"
        ]
      },
      {
        "id": 2,
        "title": "Wonder Woman",
        "genres": [
          "Action",
          "Adventure"
        ],
        "color": [
          "green"
        ],
        "platforms": [
          "MacOS"
        ]
      },
      {
        "id": 3,
        "title": "Life of Pi",
        "genres": [
          "Adventure",
          "Drama"
        ],
        "color": [
          "blue"
        ],
        "platforms": [
          "Windows"
        ]
      },
      {
        "id": 4,
        "title": "Mad Max: Fury Road",
        "genres": [
          "Adventure",
          "Science Fiction"
        ],
        "color": [
          "red"
        ],
        "platforms": [
          "MacOS",
          "Linux"
        ]
      },
      {
        "id": 5,
        "title": "Moana",
        "genres": [
          "Fantasy",
          "Action"
        ],
        "color": [
          "red"
        ],
        "platforms": [
          "Windows"
        ]
      },
      {
        "id": 6,
        "title": "Philadelphia",
        "genres": [
          "Drama"
        ],
        "color": [
          "blue"
        ],
        "platforms": [
          "MacOS",
          "Linux",
          "Windows"
        ]
      }
    ]);
    let (response, code) =
        index.update_settings_filterable_attributes(json!(["genres", "color"])).await;
    assert_eq!(202, code, "{:?}", response);
    index.wait_task(response.uid()).await;

    let (response, _code) = index.add_documents(documents, None).await;
    index.wait_task(response.uid()).await;

    let (response, code) =
        index.facet_search(json!({"facetQuery": "", "facetName": "genres", "q": "" })).await;

    assert_eq!(code, 200, "{}", response);
    snapshot!(response["facetHits"], @r###"[{"value":"Action","count":2},{"value":"Adventure","count":3},{"value":"Drama","count":3},{"value":"Fantasy","count":1},{"value":"Romance","count":1},{"value":"Science Fiction","count":1}]"###);
}

#[actix_rt::test]
async fn advanced_facet_search() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = DOCUMENTS.clone();
    index.update_settings_filterable_attributes(json!(["genres"])).await;
    index.update_settings_typo_tolerance(json!({ "enabled": false })).await;
    let (task, _status_code) = index.add_documents(documents, None).await;
    index.wait_task(task.uid()).await.succeeded();

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "adventre"})).await;

    snapshot!(code, @"200 OK");
    snapshot!(response["facetHits"].as_array().unwrap().len(), @"0");

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "àdventure"})).await;

    snapshot!(code, @"200 OK");
    snapshot!(response["facetHits"].as_array().unwrap().len(), @"1");
}

#[actix_rt::test]
async fn more_advanced_facet_search() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = DOCUMENTS.clone();
    index.update_settings_filterable_attributes(json!(["genres"])).await;
    index.update_settings_typo_tolerance(json!({ "disableOnWords": ["adventre"] })).await;
    let (task, _status_code) = index.add_documents(documents, None).await;
    index.wait_task(task.uid()).await.succeeded();

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "adventre"})).await;

    snapshot!(code, @"200 OK");
    snapshot!(response["facetHits"].as_array().unwrap().len(), @"0");

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "adventure"})).await;

    snapshot!(code, @"200 OK");
    snapshot!(response["facetHits"].as_array().unwrap().len(), @"1");
}

#[actix_rt::test]
async fn simple_facet_search_with_max_values() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = DOCUMENTS.clone();
    index.update_settings_faceting(json!({ "maxValuesPerFacet": 1 })).await;
    index.update_settings_filterable_attributes(json!(["genres"])).await;
    let (task, _status_code) = index.add_documents(documents, None).await;
    index.wait_task(task.uid()).await.succeeded();

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "a"})).await;

    assert_eq!(code, 200, "{}", response);
    assert_eq!(dbg!(response)["facetHits"].as_array().unwrap().len(), 1);
}

#[actix_rt::test]
async fn simple_facet_search_by_count_with_max_values() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = DOCUMENTS.clone();
    index
        .update_settings_faceting(
            json!({ "maxValuesPerFacet": 1, "sortFacetValuesBy": { "*": "count" } }),
        )
        .await;
    index.update_settings_filterable_attributes(json!(["genres"])).await;
    let (task, _status_code) = index.add_documents(documents, None).await;
    index.wait_task(task.uid()).await.succeeded();

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "a"})).await;

    assert_eq!(code, 200, "{}", response);
    assert_eq!(dbg!(response)["facetHits"].as_array().unwrap().len(), 1);
}

#[actix_rt::test]
async fn non_filterable_facet_search_error() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = DOCUMENTS.clone();
    let (task, _status_code) = index.add_documents(documents, None).await;
    index.wait_task(task.uid()).await.succeeded();

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "a"})).await;
    assert_eq!(code, 400, "{}", response);

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "adv"})).await;
    assert_eq!(code, 400, "{}", response);
}

#[actix_rt::test]
async fn facet_search_dont_support_words() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = DOCUMENTS.clone();
    index.update_settings_filterable_attributes(json!(["genres"])).await;
    let (task, _status_code) = index.add_documents(documents, None).await;
    index.wait_task(task.uid()).await.succeeded();

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "words"})).await;

    assert_eq!(code, 200, "{}", response);
    assert_eq!(response["facetHits"].as_array().unwrap().len(), 0);
}

#[actix_rt::test]
async fn simple_facet_search_with_sort_by_count() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = DOCUMENTS.clone();
    index.update_settings_faceting(json!({ "sortFacetValuesBy": { "*": "count" } })).await;
    index.update_settings_filterable_attributes(json!(["genres"])).await;
    let (task, _status_code) = index.add_documents(documents, None).await;
    index.wait_task(task.uid()).await.succeeded();

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "a"})).await;

    assert_eq!(code, 200, "{}", response);
    let hits = response["facetHits"].as_array().unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0], json!({ "value": "Action", "count": 3 }));
    assert_eq!(hits[1], json!({ "value": "Adventure", "count": 2 }));
}

#[actix_rt::test]
async fn add_documents_and_deactivate_facet_search() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = DOCUMENTS.clone();
    let (response, _code) = index.add_documents(documents, None).await;
    index.wait_task(response.uid()).await;
    let (response, code) = index
        .update_settings(json!({
            "facetSearch": false,
            "filterableAttributes": ["genres"],
        }))
        .await;
    assert_eq!("202", code.as_str(), "{:?}", response);
    index.wait_task(response.uid()).await;

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "a"})).await;

    assert_eq!(code, 400, "{}", response);
    snapshot!(response, @r###"
    {
      "message": "The facet search is disabled for this index",
      "code": "facet_search_disabled",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#facet_search_disabled"
    }
    "###);
}

#[actix_rt::test]
async fn deactivate_facet_search_and_add_documents() {
    let server = Server::new().await;
    let index = server.index("test");

    let (response, code) = index
        .update_settings(json!({
            "facetSearch": false,
            "filterableAttributes": ["genres"],
        }))
        .await;
    assert_eq!("202", code.as_str(), "{:?}", response);
    index.wait_task(response.uid()).await;
    let documents = DOCUMENTS.clone();
    let (response, _code) = index.add_documents(documents, None).await;
    index.wait_task(response.uid()).await;

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "a"})).await;

    assert_eq!(code, 400, "{}", response);
    snapshot!(response, @r###"
    {
      "message": "The facet search is disabled for this index",
      "code": "facet_search_disabled",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#facet_search_disabled"
    }
    "###);
}

#[actix_rt::test]
async fn deactivate_facet_search_add_documents_and_activate_facet_search() {
    let server = Server::new().await;
    let index = server.index("test");

    let (response, code) = index
        .update_settings(json!({
            "facetSearch": false,
            "filterableAttributes": ["genres"],
        }))
        .await;
    assert_eq!("202", code.as_str(), "{:?}", response);
    index.wait_task(response.uid()).await;
    let documents = DOCUMENTS.clone();
    let (response, _code) = index.add_documents(documents, None).await;
    index.wait_task(response.uid()).await;

    let (response, code) = index
        .update_settings(json!({
            "facetSearch": true,
        }))
        .await;
    assert_eq!("202", code.as_str(), "{:?}", response);
    index.wait_task(response.uid()).await;

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "a"})).await;

    assert_eq!(code, 200, "{}", response);
    assert_eq!(dbg!(response)["facetHits"].as_array().unwrap().len(), 2);
}

#[actix_rt::test]
async fn deactivate_facet_search_add_documents_and_reset_facet_search() {
    let server = Server::new().await;
    let index = server.index("test");

    let (response, code) = index
        .update_settings(json!({
            "facetSearch": false,
            "filterableAttributes": ["genres"],
        }))
        .await;
    assert_eq!("202", code.as_str(), "{:?}", response);
    index.wait_task(response.uid()).await;
    let documents = DOCUMENTS.clone();
    let (response, _code) = index.add_documents(documents, None).await;
    index.wait_task(response.uid()).await;

    let (response, code) = index
        .update_settings(json!({
            "facetSearch": serde_json::Value::Null,
        }))
        .await;
    assert_eq!("202", code.as_str(), "{:?}", response);
    index.wait_task(response.uid()).await;

    let (response, code) =
        index.facet_search(json!({"facetName": "genres", "facetQuery": "a"})).await;

    assert_eq!(code, 200, "{}", response);
    assert_eq!(dbg!(response)["facetHits"].as_array().unwrap().len(), 2);
}

#[actix_rt::test]
async fn facet_search_with_filterable_attributes_rules() {
    test_settings_documents_indexing_swapping_and_facet_search(
        &DOCUMENTS,
        &json!({"filterableAttributes": ["genres"]}),
        &json!({"facetName": "genres", "facetQuery": "a"}),
        |response, code| {
            snapshot!(code, @"200 OK");
            snapshot!(response["facetHits"], @r###"[{"value":"Action","count":3},{"value":"Adventure","count":2}]"###);
        },
    )
    .await;

    test_settings_documents_indexing_swapping_and_facet_search(
        &DOCUMENTS,
        &json!({"filterableAttributes": [{"attributePatterns": ["genres"], "features": {"facetSearch": true, "filter": {"equality": false, "comparison": false}}}]}),
        &json!({"facetName": "genres", "facetQuery": "a"}),
        |response, code| {
            snapshot!(code, @"200 OK");
            snapshot!(response["facetHits"], @r###"[{"value":"Action","count":3},{"value":"Adventure","count":2}]"###);
        },
    ).await;

    test_settings_documents_indexing_swapping_and_facet_search(
        &NESTED_DOCUMENTS,
        &json!({"filterableAttributes": ["doggos.name"]}),
        &json!({"facetName": "doggos.name", "facetQuery": "b"}),
        |response, code| {
            snapshot!(code, @"200 OK");
            snapshot!(response["facetHits"], @r###"[{"value":"bobby","count":1},{"value":"buddy","count":1}]"###);
        },
    )
    .await;

    test_settings_documents_indexing_swapping_and_facet_search(
        &NESTED_DOCUMENTS,
        &json!({"filterableAttributes": [{"attributePatterns": ["doggos.name"], "features": {"facetSearch": true, "filter": {"equality": false, "comparison": false}}}]}),
        &json!({"facetName": "doggos.name", "facetQuery": "b"}),
        |response, code| {
            snapshot!(code, @"200 OK");
            snapshot!(response["facetHits"], @r###"[{"value":"bobby","count":1},{"value":"buddy","count":1}]"###);
        },
    ).await;
}

#[actix_rt::test]
async fn facet_search_with_filterable_attributes_rules_errors() {
    test_settings_documents_indexing_swapping_and_facet_search(
        &DOCUMENTS,
        &json!({"filterableAttributes": ["genres"]}),
        &json!({"facetName": "invalid", "facetQuery": "a"}),
        |response, code| {
            snapshot!(code, @"400 Bad Request");
            snapshot!(response["message"], @r###""Attribute `invalid` is not facet-searchable. Available facet-searchable attributes are: `genres`. To make it facet-searchable add it to the `filterableAttributes` index settings.""###);
        },
    )
    .await;

    test_settings_documents_indexing_swapping_and_facet_search(
      &DOCUMENTS,
      &json!({"filterableAttributes": [{"attributePatterns": ["genres"]}]}),
      &json!({"facetName": "genres", "facetQuery": "a"}),
      |response, code| {
          snapshot!(code, @"400 Bad Request");
          snapshot!(response["message"], @r###""Attribute `genres` is not facet-searchable. This index does not have configured facet-searchable attributes. To make it facet-searchable add it to the `filterableAttributes` index settings.""###);
      },
    )
    .await;

    test_settings_documents_indexing_swapping_and_facet_search(
        &DOCUMENTS,
        &json!({"filterableAttributes": [{"attributePatterns": ["genres"], "features": {"facetSearch": false, "filter": {"equality": true, "comparison": true}}}]}),
        &json!({"facetName": "genres", "facetQuery": "a"}),
        |response, code| {
            snapshot!(code, @"400 Bad Request");
            snapshot!(response["message"], @r###""Attribute `genres` is not facet-searchable. This index does not have configured facet-searchable attributes. To make it facet-searchable add it to the `filterableAttributes` index settings.""###);
        },
    ).await;

    test_settings_documents_indexing_swapping_and_facet_search(
        &DOCUMENTS,
        &json!({"filterableAttributes": [{"attributePatterns": ["genres"], "features": {"facetSearch": false, "filter": {"equality": false, "comparison": false}}}]}),
        &json!({"facetName": "genres", "facetQuery": "a"}),
        |response, code| {
            snapshot!(code, @"400 Bad Request");
            snapshot!(response["message"], @r###""Attribute `genres` is not facet-searchable. This index does not have configured facet-searchable attributes. To make it facet-searchable add it to the `filterableAttributes` index settings.""###);
        },
    ).await;

    test_settings_documents_indexing_swapping_and_facet_search(
        &NESTED_DOCUMENTS,
        &json!({"filterableAttributes": [{"attributePatterns": ["doggos.name"]}]}),
        &json!({"facetName": "invalid.name", "facetQuery": "b"}),
        |response, code| {
            snapshot!(code, @"400 Bad Request");
            snapshot!(response["message"], @r###""Attribute `invalid.name` is not facet-searchable. This index does not have configured facet-searchable attributes. To make it facet-searchable add it to the `filterableAttributes` index settings.""###);
        },
    )
    .await;

    test_settings_documents_indexing_swapping_and_facet_search(
        &NESTED_DOCUMENTS,
        &json!({"filterableAttributes": [{"attributePatterns": ["doggos.name"], "features": {"facetSearch": false, "filter": {"equality": true, "comparison": true}}}]}),
        &json!({"facetName": "doggos.name", "facetQuery": "b"}),
        |response, code| {
            snapshot!(code, @"400 Bad Request");
            snapshot!(response["message"], @r###""Attribute `doggos.name` is not facet-searchable. This index does not have configured facet-searchable attributes. To make it facet-searchable add it to the `filterableAttributes` index settings.""###);
        },
    ).await;

    test_settings_documents_indexing_swapping_and_facet_search(
        &NESTED_DOCUMENTS,
        &json!({"filterableAttributes": [{"attributePatterns": ["doggos.name"], "features": {"facetSearch": false, "filter": {"equality": false, "comparison": false}}}]}),
        &json!({"facetName": "doggos.name", "facetQuery": "b"}),
        |response, code| {
            snapshot!(code, @"400 Bad Request");
            snapshot!(response["message"], @r###""Attribute `doggos.name` is not facet-searchable. This index does not have configured facet-searchable attributes. To make it facet-searchable add it to the `filterableAttributes` index settings.""###);
        },
    ).await;
}
