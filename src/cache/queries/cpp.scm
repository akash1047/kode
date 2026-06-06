(function_definition
  declarator: (function_declarator
    declarator: (identifier) @function))
(class_specifier name: (type_identifier) @class)
(struct_specifier name: (type_identifier) @struct)
(namespace_definition name: (namespace_identifier) @namespace)
(function_definition
  declarator: (function_declarator
    declarator: (qualified_identifier name: (identifier) @method)))
