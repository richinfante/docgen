# docgen

**NOTE: Docgen is experimental software. It's not feature complete, and doesn't work half the time. It's extremely unstable, and this repo is published mostly for feedback purposes. No not rely on this software.**

Docgen is a static site renderer which is built using servo's html5ever and spidermonkey. It aims to make static site generation to be effortless and removing templating languages such as liquid.

Docgen's template syntax is based on / inspired by the syntax used by vuejs's templates. the rationale behind this is that all templates become are normal HTML pages and do not need extra text nodes containing the conditional / template logic. (like liquid, mustache, others..). This means pages can be developed and tested in their pure template form, without needing much (or any) tooling to do so in a nice way.

Templates also allow <script> tags to be run (At compile time) with the `static` attribute set.

Please note that the generation does **NOT** currently expose the normal javascript dom api, such as `document.createElement`, etc. It is an empty javascript context only used for templating. If this feature would be useful to you, file an issue with use cases / info.

## Feature Roadmap
these are features I'd like to have initially, in no particular order.
- [x] add html parser
- [x] add js engine
- [x] conditional logic with `x-if` (tentative name)
- [x] attribute variable expansion (`:href="link.title"` with `link = { title: 'HI' }` -> `href="HI"`)
- [x] layout includes via `layout` js variable and `x-content-slot` attribute.
- [x] iteration logic with `x-each` (tentative name) **Experimentally Implemented**
- [ ] conditional css class generation (similar to vuejs's :class attribute).
- [x] html partials via `x-include` attribute.
- [ ] way to extract the contents of a div to replace it. Potentially called `x-extract`
- [ ] json/yaml/etc data file loading for configuration / data.
- [ ] markdown support with front-matter data + rendering (similar to jekyll)
- [ ] page-fork rendering: instead of iterating a page via `x-each`, render multiple copies of a page with different elements. To be used for dynamic tagging.
- [ ] helper application to call the main `docgen` binary for site generation. This is the template engine, from which the actual site generator will discover supported template pages and render them for upload.

## Building from source
```bash
brew install yasm autoconf
set AUTOCONF="$(which autoconf)"
cargo build
```

## Runing example
```bash
cargo run -- -i examples/demo.html
```

## Process
1. render the page to html if it's markdown.
2. render using the main `docgen` process, in a top-down manner.
  - evaluate any `script static` tags.
  - replace all template variables

## Template Examples
At the moment, docgen only produces processes html templates. This will change in the future, with options for markdown, etc.

### Source Template
```html
<html>
<head>
  <script static>
  let links = [{
    href: 'https://google.com',
    title: 'google'
  }, {
    href: 'https://apple.com',
    title: 'apple'
  },{
    href: 'https://amazon.com',
    title: 'amazon'
  }]
  </script>
</head>
<body>
  <ul>
    <li x-each="links" x-as="link">
      <a :href="link.href">{{link.title}}</a>
    </li>
  </ul>
</body>
</html>
```

### Result Output
```html
<html>
<head>
</head>
<body>
  <ul>
    <li>
      <a href="https://google.com">google</a>
    </li><li>
      <a href="https://apple.com">apple</a>
    </li><li>
      <a href="https://amazon.com">amazon</a>
    </li>
  </ul>
</body>
</html>
```

## Converting from liquid
### If statements
```html
{% if variable_name %}
<span>Hello, World!</span>
{% endif %}
```

```html
<span x-if="variable_name">Hello, World</span>
```

### Attribute bindings
```html
<span class="{{className}}">Test</span>
```

Note: the `:class` binding *may* be updated in the future to allow a dictionary, which in turn renders to a space-separated class string based on all the keys that have truthy values. (equivalent to: `Object.entries(classDict).filter(el => el[1]).map(el => el[0]).join(' ')`)

```html
<span :class="className">Test</span>
```


### For loops
note: for loops are not feature complete. All for loops currently bind to `item` and the syntax is likely to change.
```html
<ul>
{% for item in items %}
  <li>
    <a href="{{item.url}}">{{item.title}}</a>
  </li>
{% endfor %}
</ul>
```

```html
<ul>
  <li x-for="items" x-as="item">
    <a :href="item.url">{{item.title}}</a>
  </li>
</ul>
```

### Template Extension
**hello.md**
```md
<script>
layout = './base.html'
title = 'Example'
</script>

# Hello, World
```

**base.html**
```html
<html>
  <head>{{child.title}}</head>
  <body>
    <div x-content-slot></div>
  </body>
</html>
```

**out.html**
```html
<html>
  <head>Example</head>
  <body>
    <h1>Hello, World</h1>
  </body>
</html>
```

### HTML Include
This currently only works with HTML files.
In a later revision, they will work with all supported types.

```md
<div x-include="./example.html"></div>
```

