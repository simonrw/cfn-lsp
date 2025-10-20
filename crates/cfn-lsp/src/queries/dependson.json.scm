(pair
  key: (string
         (string_content) @fn.tag)
  value: (string
           (string_content) @fn.target)
    (#eq? @fn.tag "DependsOn")) @fn

(pair
  key: (string
         (string_content) @arr.tag)
  value: (array
           (string
             (string_content) @arr.target))
    (#eq? @arr.tag "DependsOn")) @arr
