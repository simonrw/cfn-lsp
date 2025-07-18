;; Simple case
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @key))
  value: (flow_node
           (tag) @tag
           (double_quote_scalar) @value)
  (#eq? @tag "!Sub"))
