<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html
-->
<script lang="ts">
    import type { TableDatum } from "./graph-helpers";
    import { direction } from "../lib/i18n";
    export let tableData: TableDatum[];
</script>

<div>
    <table dir={direction()}>
        {#each tableData as { label, value }}
            <tr>
                <td class="align-end">{label}:</td>
                <td class="align-start">{value}</td>
            </tr>
        {/each}
    </table>
</div>

<style lang="scss">
    div {
        display: flex;
        justify-content: center;
    }

    td {
        padding: 3px;
    }

    .align-end {
        text-align: end;
    }

    .align-start {
        text-align: start;
    }
</style>
