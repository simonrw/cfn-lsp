(pair
  key: (string
         (string_content) @fn.tag)
  value: (array
           .
           (string
             (string_content) @fn.target))
    (#eq? @fn.tag "Fn::GetAtt")) @fn
