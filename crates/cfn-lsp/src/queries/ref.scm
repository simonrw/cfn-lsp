(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @fn.tag))
  value: (_)+  @fn.target
    (#eq? @fn.tag "Ref")) @fn

(flow_node
  (tag) @tag.tag
  (_)+ @tag.target
  (#eq? @tag.tag "!Ref")) @tag
