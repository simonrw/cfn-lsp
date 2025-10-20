;; Block mapping form with flow sequence: Fn::GetAtt: [Name, Property]
;; We want to capture only the first element (the resource name)
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @fn.tag))
  value: (flow_node
           (flow_sequence
             .
             (flow_node
               (_) @fn.target)))
    (#eq? @fn.tag "Fn::GetAtt")) @fn

;; Block mapping form with block sequence: Fn::GetAtt:\n  - Name\n  - Property
;; We want to capture only the first element (the resource name)
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @block.tag))
  value: (block_node
           (block_sequence
             .
             (block_sequence_item
               (flow_node
                 (_) @block.target))))
    (#eq? @block.tag "Fn::GetAtt")) @block

;; Tag form: !GetAtt Name.Property
(flow_node
  (tag) @tag.tag
  (plain_scalar
    (string_scalar) @tag.value)
  (#eq? @tag.tag "!GetAtt")) @tag
