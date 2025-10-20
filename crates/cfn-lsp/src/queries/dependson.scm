;; DependsOn with a single resource name (scalar value)
;; DependsOn: ResourceName
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @scalar.tag))
  value: (flow_node
           (_) @scalar.target)
    (#eq? @scalar.tag "DependsOn")) @scalar

;; DependsOn with flow sequence: DependsOn: [Resource1, Resource2]
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @flow.tag))
  value: (flow_node
           (flow_sequence
             (flow_node
               (_) @flow.target)))
    (#eq? @flow.tag "DependsOn")) @flow

;; DependsOn with block sequence: DependsOn:\n  - Resource1\n  - Resource2
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @block.tag))
  value: (block_node
           (block_sequence
             (block_sequence_item
               (flow_node
                 (_) @block.target))))
    (#eq? @block.tag "DependsOn")) @block
