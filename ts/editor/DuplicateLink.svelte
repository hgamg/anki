<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html
-->
<script lang="ts">
    import * as tr from "../lib/ftl";
    import { bridgeCommand } from "../lib/bridgecommand";
</script>

<a
    class="duplicate-link"
    href="/#"
    tabindex="-1"
    on:click={() => bridgeCommand("dupes")}
>
    {tr.editingShowDuplicates()}
</a>

<style lang="scss">
    .duplicate-link {
        color: var(--highlight-color);
        text-align: center;
        flex-grow: 1;
    }
</style>
