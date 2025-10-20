;; Block mapping form with flow sequence: Fn::If: [ConditionName, ValueIfTrue, ValueIfFalse]
;; We want to capture only the first element (the condition name)
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @fn.tag))
  value: (flow_node
           (flow_sequence
             .
             (flow_node
               (_) @fn.target)))
    (#eq? @fn.tag "Fn::If")) @fn

;; Block mapping form with block sequence: Fn::If:\n  - ConditionName\n  - ValueIfTrue\n  - ValueIfFalse
;; We want to capture only the first element (the condition name)
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
    (#eq? @block.tag "Fn::If")) @block

;; Tag form: !If [ConditionName, ValueIfTrue, ValueIfFalse]
;; We want to capture only the first element (the condition name)
(flow_node
  (tag) @tag.tag
  (flow_sequence
    .
    (flow_node
      (_) @tag.target))
  (#eq? @tag.tag "!If")) @tag

;; Tag form with block sequence: !If\n  - ConditionName\n  - ValueIfTrue\n  - ValueIfFalse
(block_node
  (tag) @tagblock.tag
  (block_sequence
    .
    (block_sequence_item
      (flow_node
        (_) @tagblock.target)))
  (#eq? @tagblock.tag "!If")) @tagblock
