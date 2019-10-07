
<!-- in the child... -->
<layout src="./server/src/html/base.html">
  <script slot="javascript" src="https://richinfante.com/example.js"></script>
</layout>

<!-- in the parent -->
<slot name="javascript-head" multiple></slot>
<slot name="javascript" multiple></slot>
<slot name="css" multiple></slot>