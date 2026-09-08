///|
struct Product {
  name : String
  price : Int
} derive(Eq)

fn line_item(product : Product, quantity : Int, builds : @ref.Ref[Int]) -> @ui.Element {
  builds.val += 1
  let initial = quantity
  @ui.div([
    @ui.text(product.name),
    @ui.text("\{quantity}"),
    @ui.text("\{product.price * quantity}"),
    @ui.text("\{initial}"),
  ])
}

fn forwarded_item(product : Product, quantity : Int, builds : @ref.Ref[Int]) -> @ui.Element {
  line_item(product, quantity, builds)
}

///|
test "plain component props retain nodes, setup snapshots, and scoped subscriptions" {
  let baseline = @reactive.active_effect_count()
  @native.test_tree(fn(root) raise {
    let (quantity, set_quantity) = @ui.create_signal(1)
    let (product, set_product) = @ui.create_signal(Product::{ name: "Mug", price: 12 })
    let builds = @ref.Ref(0)
    let node = forwarded_item(product(), quantity(), builds).native_node()
    root.append(node)
    let labels = node.children().map(child => child.children()[0])
    assert_eq(labels.map(label => label.text_content()), ["Mug", "1", "12", "1"])
    ignore(root.take_batch())
    set_quantity(3)
    assert_eq(builds.val, 1)
    assert_eq(labels.map(label => label.text_content()), ["Mug", "3", "36", "1"])
    assert_eq(root.pending_mutations(), 2)
    ignore(root.take_batch())
    set_product(Product::{ name: "Cup", price: 10 })
    assert_eq(labels.map(label => label.text_content()), ["Cup", "3", "30", "1"])
    assert_eq(root.pending_mutations(), 2)
    for index, child in node.children() {
      assert_true(child.children()[0] == labels[index])
    }
    node.remove()
    ignore(root.take_batch())
    set_quantity(4)
    assert_eq(root.pending_mutations(), 0)
  })
  assert_eq(@reactive.active_effect_count(), baseline)
}

///|
test "imported component props retain named argument bindings" {
  @native.test_tree(fn(root) raise {
    let (quantity, set_quantity) = @ui.create_signal(1)
    let node = @parts.label("Mug", quantity=quantity()).native_node()
    root.append(node)
    let label = node.children()[1].children()[0]
    ignore(root.take_batch())
    set_quantity(2)
    assert_eq(label.text_content(), "2")
    assert_true(node.children()[1].children()[0] == label)
    assert_eq(root.pending_mutations(), 1)
  })
}
