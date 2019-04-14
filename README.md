# docgen
Docgen is a static site renderer which is built using servo's html5ever and spidermonkey. It aims to make static site generation to be effortless and removing templating languages such as liquid.

Docgen's template syntax is based on / inspired by the syntax used by vuejs's templates. the rationale behind this is that all templates become valid HTML pages and do not need extra text nodes containing the conditional logic. (like liquid, mustache, others..). This means pages can be developed and tested in their pure template form, without needing much (or any) tooling to do so in a nice way.

**NOTE: Docgen is experimental software. It's not feature complete, and doesn't work half the time. It's extremely unstable, and this repo is published mostly for feedback purposes.**

## Feature Roadmap
these are features I'd like to have, in no particular order.
- [x] add html parser
- [x] add js engine
- [x] conditional logic with `x-if` (tentative name)
- [x] attribute variable expansion (`:href="link.title"` with `link = { title: 'HI' }` -> `href="HI"`)
- [ ] iteration logic with `x-each` (tentative name) **Experimentally Implemented**
- [ ] conditional css class generation (similar to vuejs's :class attribute).
- [ ] json/yaml/etc data file loading for configuration / data.
- [ ] markdown support with front-matter data + rendering (similar to jekyll)
- [ ] page-fork rendering: instead of iterating a page via `x-each`, render multiple copies of a page with different elements. To be used for dynamic tagging.

## Build
```bash
brew install yasm
set AUTOCONF="$(which autoconf)"
cargo build
```

## Run
```bash
cat demo.html | cargo run
```

## Template Examples
At the moment, docgen only produces processes pure html templates. This will change in the future, with options for markdown, etc.

### Source Template
```html
<html>
<head>
  <script ssr="true">
  x = 1337;
  let SECRET_ENABLED = true;
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
    <!-- TODO: allow binding to different variables. -->
    <!-- Currently, only binds to "item" which makes nesting not work -->
    <li x-each="links">
      <a :href="item.href">{{item.title}}</a>
    </li>
  </ul>
</body>
</html>
```

### Result Output
```html
<html>
<head>
  <!-- TODO: remove ssr scripts when rendering -->
  <script ssr="true">
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
    <!-- TODO: remove iteration attributes -->
    <li x-each="links">
      <a href="https://google.com">google</a>
    </li><li x-each="links">
      <a href="https://apple.com">apple</a>
    </li><li x-each="links">
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

Note: the `:class` binding *may* be updated in the future to allow a dictionary, which in turn renders to a space-separated class string based on all the keys that have truthy values.
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
  <li x-for="items">
    <a :href="item.url">{{item.title}}</a>
  </li>
</ul>
```
