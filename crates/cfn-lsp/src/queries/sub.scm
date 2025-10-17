;; Block mapping form: Fn::Sub: "string"
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @fn.tag))
  value: (flow_node
           (double_quote_scalar) @fn.target)
    (#eq? @fn.tag "Fn::Sub")) @fn

;; Tag form: !Fn::Sub "string"
(flow_node
  (tag) @tag.tag
  (double_quote_scalar) @tag.target
  (#eq? @tag.tag "!Fn::Sub")) @tag
