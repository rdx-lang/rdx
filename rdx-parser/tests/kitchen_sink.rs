use rdx_parser::*;

const EXAMPLE: &str = include_str!("example.rdx");

// ==================== Helpers ====================

fn find_nodes<'a>(nodes: &'a [Node], pred: &dyn Fn(&Node) -> bool) -> Vec<&'a Node> {
    let mut found = Vec::new();
    for node in nodes {
        if pred(node) {
            found.push(node);
        }
        if let Some(children) = node.children() {
            found.extend(find_nodes(children, pred));
        }
    }
    found
}

fn find_components<'a>(nodes: &'a [Node]) -> Vec<&'a ComponentNode> {
    let all = find_nodes(nodes, &|n| matches!(n, Node::Component(_)));
    all.into_iter()
        .filter_map(|n| {
            if let Node::Component(c) = n {
                Some(c)
            } else {
                None
            }
        })
        .collect()
}

fn find_variables<'a>(nodes: &'a [Node]) -> Vec<&'a VariableNode> {
    let all = find_nodes(nodes, &|n| matches!(n, Node::Variable(_)));
    all.into_iter()
        .filter_map(|n| {
            if let Node::Variable(v) = n {
                Some(v)
            } else {
                None
            }
        })
        .collect()
}

fn find_errors<'a>(nodes: &'a [Node]) -> Vec<&'a ErrorNode> {
    let all = find_nodes(nodes, &|n| matches!(n, Node::Error(_)));
    all.into_iter()
        .filter_map(|n| {
            if let Node::Error(e) = n {
                Some(e)
            } else {
                None
            }
        })
        .collect()
}

// ==================== Tests ====================

#[test]
fn parses_without_errors() {
    let root = parse(EXAMPLE);
    let errors = find_errors(&root.children);
    assert!(
        errors.is_empty(),
        "Should parse with zero errors, got: {:#?}",
        errors
    );
}

#[test]
fn frontmatter_extracted() {
    let root = parse(EXAMPLE);
    let fm = root.frontmatter.as_ref().expect("Should have frontmatter");
    assert_eq!(fm["title"], "RDX Kitchen Sink");
    assert_eq!(fm["version"], 2.1);
    assert_eq!(fm["author"], "Jane Doe");
    let tags = fm["tags"].as_array().expect("tags should be array");
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0], "documentation");
    assert_eq!(tags[1], "components");
}

#[test]
fn root_position() {
    let root = parse(EXAMPLE);
    assert_eq!(root.position.start.line, 1);
    assert_eq!(root.position.start.column, 1);
    assert_eq!(root.position.start.offset, 0);
    assert_eq!(root.position.end.offset, EXAMPLE.len());
}

// --- Self-closing components ---

#[test]
fn hero_image_component() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let hero = comps
        .iter()
        .find(|c| c.name == "HeroImage")
        .expect("HeroImage not found");
    assert!(!hero.is_inline);
    assert!(hero.children.is_empty());
    assert_eq!(hero.attributes.len(), 2);
    assert_eq!(hero.attributes[0].name, "src");
    assert_eq!(
        hero.attributes[0].value,
        AttributeValue::String("/assets/hero.png".into())
    );
    assert_eq!(hero.attributes[1].name, "alt");
    assert_eq!(
        hero.attributes[1].value,
        AttributeValue::String("Hero banner".into())
    );
}

#[test]
fn divider_no_space_before_close() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let divider = comps
        .iter()
        .find(|c| c.name == "Divider")
        .expect("Divider not found");
    assert!(divider.children.is_empty());
    assert!(divider.attributes.is_empty());
}

#[test]
fn spacer_with_number_attr() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let spacer = comps
        .iter()
        .find(|c| c.name == "Spacer")
        .expect("Spacer not found");
    assert_eq!(spacer.attributes[0].name, "height");
    assert_eq!(
        spacer.attributes[0].value,
        AttributeValue::Number(64.into())
    );
}

// --- Block components with children ---

#[test]
fn notice_component() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let notice = comps
        .iter()
        .find(|c| c.name == "Notice")
        .expect("Notice not found");
    assert!(!notice.is_inline);
    assert_eq!(notice.attributes.len(), 2);
    assert_eq!(notice.attributes[0].name, "type");
    assert_eq!(
        notice.attributes[0].value,
        AttributeValue::String("warning".into())
    );
    assert_eq!(notice.attributes[1].name, "dismissible");
    assert_eq!(notice.attributes[1].value, AttributeValue::Bool(true)); // boolean shorthand
    assert!(!notice.children.is_empty(), "Notice should have children");

    // Children should contain a variable interpolation for {$version}
    let vars = find_variables(&notice.children);
    let has_version = vars.iter().any(|v| v.path == "version");
    assert!(
        has_version,
        "Notice should contain {{$version}}: {:?}",
        vars
    );
}

#[test]
fn card_with_nested_badge() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let card = comps
        .iter()
        .find(|c| c.name == "Card")
        .expect("Card not found");
    assert_eq!(card.attributes[0].name, "title");
    assert_eq!(
        card.attributes[0].value,
        AttributeValue::String("Getting Started".into())
    );
    assert_eq!(card.attributes[1].name, "theme");
    assert_eq!(
        card.attributes[1].value,
        AttributeValue::String("dark".into())
    );

    // Should contain a nested Badge self-closing component
    let nested = find_components(&card.children);
    let badge = nested
        .iter()
        .find(|c| c.name == "Badge")
        .expect("Nested Badge not found");
    assert_eq!(
        badge.attributes[0].value,
        AttributeValue::String("new".into())
    );
}

// --- Deeply nested components ---

#[test]
fn deeply_nested_layout() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let layout = comps
        .iter()
        .find(|c| c.name == "Layout")
        .expect("Layout not found");
    assert_eq!(
        layout.attributes[0].value,
        AttributeValue::String("horizontal".into())
    );

    let layout_children = find_components(&layout.children);
    let sidebar = layout_children
        .iter()
        .find(|c| c.name == "Sidebar")
        .expect("Sidebar not found");
    assert_eq!(sidebar.attributes[0].name, "width");
    assert_eq!(
        sidebar.attributes[0].value,
        AttributeValue::Number(240.into())
    );

    let sidebar_children = find_components(&sidebar.children);
    let nav_group = sidebar_children
        .iter()
        .find(|c| c.name == "NavGroup")
        .expect("NavGroup not found");
    assert_eq!(
        nav_group.attributes[0].value,
        AttributeValue::String("Docs".into())
    );

    let nav_items = find_components(&nav_group.children);
    assert_eq!(nav_items.len(), 2);
    assert_eq!(nav_items[0].name, "NavItem");
    assert_eq!(nav_items[0].attributes[1].name, "active");
    assert_eq!(nav_items[0].attributes[1].value, AttributeValue::Bool(true));
    assert_eq!(
        nav_items[1].attributes[1].value,
        AttributeValue::Bool(false)
    );

    let main = layout_children
        .iter()
        .find(|c| c.name == "MainContent")
        .expect("MainContent not found");
    assert!(!main.children.is_empty());
}

// --- Attribute types ---

#[test]
fn string_attr_double_and_single_quotes() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let button = comps
        .iter()
        .find(|c| {
            c.name == "Button"
                && c.attributes.iter().any(|a| {
                    a.name == "label" && a.value == AttributeValue::String("Click Me".into())
                })
        })
        .expect("Button with label='Click Me' not found");
    assert_eq!(button.attributes[1].name, "theme");
    assert_eq!(
        button.attributes[1].value,
        AttributeValue::String("dark".into())
    );
}

#[test]
fn string_escaping_backslash() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let tooltips: Vec<_> = comps.iter().filter(|c| c.name == "Tooltip").collect();
    assert!(tooltips.len() >= 3, "Should have at least 3 Tooltips");

    // <Tooltip text="She said \"hello\"" />
    let t1 = tooltips[0];
    assert_eq!(
        t1.attributes[0].value,
        AttributeValue::String("She said \"hello\"".into())
    );

    // <Tooltip text='It\'s a beautiful day' />
    let t2 = tooltips[1];
    assert_eq!(
        t2.attributes[0].value,
        AttributeValue::String("It's a beautiful day".into())
    );

    // <Tooltip text='She said "hello"' />
    let t3 = tooltips[2];
    assert_eq!(
        t3.attributes[0].value,
        AttributeValue::String("She said \"hello\"".into())
    );
}

#[test]
fn primitive_attributes() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let pagination = comps
        .iter()
        .find(|c| c.name == "Pagination")
        .expect("Pagination not found");
    assert_eq!(pagination.attributes[0].name, "activePage");
    assert_eq!(
        pagination.attributes[0].value,
        AttributeValue::Number(2.into())
    );
    assert_eq!(pagination.attributes[1].value, AttributeValue::Bool(true));
    assert_eq!(pagination.attributes[2].value, AttributeValue::Bool(false));
    assert_eq!(pagination.attributes[3].value, AttributeValue::Null);

    let slider = comps
        .iter()
        .find(|c| c.name == "Slider")
        .expect("Slider not found");
    match &slider.attributes[0].value {
        AttributeValue::Number(n) => assert!((n.as_f64().unwrap() - (-3.14)).abs() < f64::EPSILON),
        other => panic!("Expected number, got {:?}", other),
    }
    match &slider.attributes[1].value {
        AttributeValue::Number(n) => assert!((n.as_f64().unwrap() - 2.5e10).abs() < 1.0),
        other => panic!("Expected number, got {:?}", other),
    }
}

#[test]
fn boolean_shorthand_attributes() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let input = comps
        .iter()
        .find(|c| c.name == "Input")
        .expect("Input not found");
    assert_eq!(input.attributes[0].name, "disabled");
    assert_eq!(input.attributes[0].value, AttributeValue::Bool(true));

    let checkbox = comps
        .iter()
        .find(|c| c.name == "Checkbox")
        .expect("Checkbox not found");
    assert_eq!(checkbox.attributes.len(), 2);
    assert_eq!(checkbox.attributes[0].name, "checked");
    assert_eq!(checkbox.attributes[1].name, "required");
}

#[test]
fn json_object_attribute() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let chart = comps
        .iter()
        .find(|c| c.name == "Chart")
        .expect("Chart not found");
    match &chart.attributes[0].value {
        AttributeValue::Object(map) => {
            assert_eq!(map["type"], serde_json::Value::String("bar".into()));
            assert_eq!(
                map["title"],
                serde_json::Value::String("Monthly Revenue".into())
            );
            let data = map["data"].as_array().unwrap();
            assert_eq!(
                data,
                &vec![
                    serde_json::Value::from(10),
                    serde_json::Value::from(20),
                    serde_json::Value::from(30),
                    serde_json::Value::from(40),
                ]
            );
            let options = map["options"].as_object().unwrap();
            assert_eq!(options["legend"], serde_json::Value::Bool(true));
            assert_eq!(options["animate"], serde_json::Value::Bool(false));
        }
        other => panic!("Expected object, got {:?}", other),
    }
}

#[test]
fn json_array_attributes() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);

    let tag_list = comps
        .iter()
        .find(|c| c.name == "TagList")
        .expect("TagList not found");
    match &tag_list.attributes[0].value {
        AttributeValue::Array(arr) => {
            assert_eq!(
                arr,
                &vec![
                    serde_json::Value::from("alpha"),
                    serde_json::Value::from("beta"),
                    serde_json::Value::from("gamma"),
                ]
            );
        }
        other => panic!("Expected array, got {:?}", other),
    }

    let data_grid = comps
        .iter()
        .find(|c| c.name == "DataGrid")
        .expect("DataGrid not found");
    match &data_grid.attributes[0].value {
        AttributeValue::Array(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0]["id"], 1);
            assert_eq!(arr[0]["name"], "Alice");
        }
        other => panic!("Expected array, got {:?}", other),
    }
}

#[test]
fn variable_attributes() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);

    let btn = comps
        .iter()
        .find(|c| {
            c.name == "Button"
                && c.attributes
                    .iter()
                    .any(|a| a.name == "label" && matches!(&a.value, AttributeValue::Variable(_)))
        })
        .expect("Button with variable label not found");
    match &btn.attributes[0].value {
        AttributeValue::Variable(v) => assert_eq!(v.path, "frontmatter.buttonText"),
        other => panic!("Expected variable, got {:?}", other),
    }

    let heading = comps
        .iter()
        .find(|c| c.name == "Heading" && c.attributes.iter().any(|a| a.name == "level"))
        .expect("Heading with variable level not found");
    match &heading.attributes[0].value {
        AttributeValue::Variable(v) => assert_eq!(v.path, "config.heading_level"),
        other => panic!("Expected variable, got {:?}", other),
    }
}

#[test]
fn mixed_attributes_multiline() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let dt = comps
        .iter()
        .find(|c| c.name == "DataTable")
        .expect("DataTable not found");
    assert_eq!(dt.attributes.len(), 5);
    assert_eq!(dt.attributes[0].name, "title");
    assert_eq!(
        dt.attributes[0].value,
        AttributeValue::String("User List".into())
    );
    assert_eq!(dt.attributes[1].name, "page");
    assert_eq!(dt.attributes[1].value, AttributeValue::Number(1.into()));
    assert_eq!(dt.attributes[2].name, "sortable");
    assert_eq!(dt.attributes[2].value, AttributeValue::Bool(true));
    assert!(matches!(&dt.attributes[3].value, AttributeValue::Array(_)));
    assert!(
        matches!(&dt.attributes[4].value, AttributeValue::Variable(v) if v.path == "handlers.sort_users")
    );
}

// --- Variable interpolation ---

#[test]
fn variable_interpolation_in_text() {
    let root = parse(EXAMPLE);
    let vars = find_variables(&root.children);
    let paths: Vec<&str> = vars.iter().map(|v| v.path.as_str()).collect();
    assert!(
        paths.contains(&"title"),
        "Should contain $title: {:?}",
        paths
    );
    assert!(
        paths.contains(&"version"),
        "Should contain $version: {:?}",
        paths
    );
    assert!(
        paths.contains(&"author"),
        "Should contain $author: {:?}",
        paths
    );
    assert!(
        paths.contains(&"config.theme_name"),
        "Should contain $config.theme_name: {:?}",
        paths
    );
}

// --- Escaping ---

#[test]
fn escape_sequences() {
    let root = parse(EXAMPLE);
    // Escaped variables should NOT produce Variable nodes
    // \{$not_a_variable} should be literal text
    let vars = find_variables(&root.children);
    let has_not_a_var = vars.iter().any(|v| v.path == "not_a_variable");
    assert!(
        !has_not_a_var,
        "Escaped variable should not be interpolated"
    );
}

// --- Code constructs ---

#[test]
fn inline_code_no_interpolation() {
    let root = parse(EXAMPLE);
    let code_inlines = find_nodes(&root.children, &|n| matches!(n, Node::CodeInline(_)));
    let has_literal = code_inlines.iter().any(|n| {
        if let Node::CodeInline(t) = n {
            t.value.contains("{$title}")
        } else {
            false
        }
    });
    assert!(has_literal, "Inline code should contain literal {{$title}}");
}

#[test]
fn fenced_code_block_no_interpolation() {
    let root = parse(EXAMPLE);
    let code_blocks = find_nodes(&root.children, &|n| matches!(n, Node::CodeBlock(_)));
    assert!(!code_blocks.is_empty(), "Should have code blocks");

    let has_literal = code_blocks.iter().any(|n| {
        if let Node::CodeBlock(t) = n {
            t.value.contains("{$this_is_not_interpolated}")
        } else {
            false
        }
    });
    assert!(
        has_literal,
        "Code block should contain literal variable syntax"
    );
}

// --- Inline components ---

#[test]
fn inline_badge_in_paragraph() {
    let root = parse(EXAMPLE);
    // "This paragraph contains an inline <Badge status="new" /> component mid-sentence."
    let comps = find_components(&root.children);
    let inline_badges: Vec<_> = comps
        .iter()
        .filter(|c| c.name == "Badge" && c.is_inline)
        .collect();
    assert!(
        !inline_badges.is_empty(),
        "Should have inline Badge components"
    );
}

#[test]
fn inline_icon_in_paragraph() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let icons: Vec<_> = comps.iter().filter(|c| c.name == "Icon").collect();
    assert!(!icons.is_empty(), "Should have Icon component");
    let icon = icons[0];
    assert!(icon.is_inline, "Icon should be inline");
    assert_eq!(
        icon.attributes[0].value,
        AttributeValue::String("check".into())
    );
    assert_eq!(icon.attributes[1].value, AttributeValue::Number(16.into()));
}

// --- Link component ---

#[test]
fn link_block_component() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let link = comps
        .iter()
        .find(|c| c.name == "Link")
        .expect("Link component not found");
    assert_eq!(
        link.attributes[0].value,
        AttributeValue::String("https://example.com".into())
    );
    assert_eq!(
        link.attributes[1].value,
        AttributeValue::String("_blank".into())
    );
    assert!(!link.children.is_empty());
}

// --- NavItem with variable attribute ---

#[test]
fn nav_item_with_variable_href() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    // Find NavItem with href={$config.docs_url}
    let nav = comps.iter().find(|c| {
        c.name == "NavItem" && c.attributes.iter().any(|a| {
            a.name == "href"
                && matches!(&a.value, AttributeValue::Variable(v) if v.path == "config.docs_url")
        })
    });
    assert!(nav.is_some(), "Should have NavItem with variable href");
}

// --- Comprehensive: total component count ---

#[test]
fn total_component_count() {
    let root = parse(EXAMPLE);
    let comps = find_components(&root.children);
    let names: Vec<&str> = comps.iter().map(|c| c.name.as_str()).collect();
    // Expected components from example.rdx:
    // HeroImage, Divider, Spacer, Notice, Card, Badge(nested), Layout, Sidebar, NavGroup,
    // NavItem x2, MainContent, Button, Tooltip x3, Pagination, Slider, Input, Checkbox,
    // Chart, TagList, DataGrid, Button(var), Heading(var), DataTable, Link, NavItem(var),
    // Badge(inline), Icon(inline)
    assert!(
        comps.len() >= 25,
        "Expected at least 25 components, got {}: {:?}",
        comps.len(),
        names
    );
}
