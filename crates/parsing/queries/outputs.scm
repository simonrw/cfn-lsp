(block_mapping
  (block_mapping_pair
    key: (flow_node
           (plain_scalar
             (string_scalar) @block.name
             (#eq? @block.name "Outputs")))
    value: (block_node
             (block_mapping
               (block_mapping_pair
                 key: (flow_node
                        (plain_scalar
                          (string_scalar) @resource.name)))))))
