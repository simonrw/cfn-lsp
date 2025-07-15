;; shorthand
(flow_node
  (tag) @tag
  (#eq? @tag "!Ref")
  (plain_scalar
    (string_scalar) @source.ref)) 
