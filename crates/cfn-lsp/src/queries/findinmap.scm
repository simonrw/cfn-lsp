;; Block mapping form with flow sequence: Fn::FindInMap: [MapName, Key1, Key2]
;; We want to capture only the first element (the mapping name)
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @fn.tag))
  value: (flow_node
           (flow_sequence
             .
             (flow_node
               (_) @fn.target)))
    (#eq? @fn.tag "Fn::FindInMap")) @fn

;; Block mapping form with block sequence: Fn::FindInMap:\n  - MapName\n  - Key1\n  - Key2
;; We want to capture only the first element (the mapping name)
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
    (#eq? @block.tag "Fn::FindInMap")) @block

;; Tag form: !FindInMap [MapName, Key1, Key2]
;; We want to capture only the first element (the mapping name)
(flow_node
  (tag) @tag.tag
  (flow_sequence
    .
    (flow_node
      (_) @tag.target))
  (#eq? @tag.tag "!FindInMap")) @tag

;; Tag form with block sequence: !FindInMap\n  - MapName\n  - Key1\n  - Key2
(block_node
  (tag) @tagblock.tag
  (block_sequence
    .
    (block_sequence_item
      (flow_node
        (_) @tagblock.target)))
  (#eq? @tagblock.tag "!FindInMap")) @tagblock
