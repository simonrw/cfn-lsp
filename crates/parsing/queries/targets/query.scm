(block_mapping
  (block_mapping_pair
    key: (flow_node
           (plain_scalar
             (string_scalar) @tag
             (#any-of? @tag 
              "Resources"
              "Outputs"
              "Parameters"
              "Mappings")))
    value: (block_node
             (block_mapping
               (block_mapping_pair
                 key: (flow_node
                        (plain_scalar
                          (string_scalar) @resource.name)))))))
