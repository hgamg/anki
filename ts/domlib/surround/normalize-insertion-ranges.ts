// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

import { findBefore, findAfter } from "./find-adjacent";
import { findWithin, findWithinNode } from "./find-within";
import { MatchResult } from "./matcher";
import type {
    FoundMatch,
    ElementMatcher,
    ElementClearer,
    FoundAdjacent,
} from "./matcher";
import type { ChildNodeRange } from "./child-node-range";

function countChildNodesRespectiveToParent(parent: Node, element: Element): number {
    return element.parentNode === parent ? element.childNodes.length : 1;
}

interface NormalizationResult {
    normalizedRanges: ChildNodeRange[];
    removedNodes: Element[];
}

function normalizeWithinInner(
    node: Element,
    parent: Node,
    removedNodes: Element[],
    matcher: ElementMatcher,
    clearer: ElementClearer,
) {
    const matches = findWithinNode(node, matcher);
    const processFoundMatches = ({ element, matchType }: FoundMatch) =>
        matchType === MatchResult.MATCH ?? clearer(element);

    for (const { element: found } of matches.filter(processFoundMatches)) {
        removedNodes.push(found);
        found.replaceWith(...found.childNodes);
    }

    /**
     * Normalization here is vital so that the
     * original range can selected afterwards
     */
    node.normalize();
    return countChildNodesRespectiveToParent(parent, node);
}

function normalizeAdjacent(
    matches: FoundAdjacent[],
    parent: Node,
    removedNodes: Element[],
    matcher: ElementMatcher,
    clearer: ElementClearer,
): number {
    let childCount = 0;
    let keepChildCount = 0;

    for (const { element, matchType } of matches) {
        switch (matchType) {
            case MatchResult.MATCH:
                childCount += normalizeWithinInner(
                    element as Element,
                    parent,
                    removedNodes,
                    matcher,
                    clearer,
                );

                removedNodes.push(element as Element);
                element.replaceWith(...element.childNodes);
                break;

            case MatchResult.KEEP:
                keepChildCount = normalizeWithinInner(
                    element as Element,
                    parent,
                    removedNodes,
                    matcher,
                    clearer,
                );

                if (clearer(element as Element)) {
                    removedNodes.push(element as Element);
                    element.replaceWith(...element.childNodes);
                    childCount += keepChildCount;
                } else {
                    childCount++;
                }
                break;

            case MatchResult.ALONG:
                childCount++;
                break;
        }
    }

    return childCount;
}

function normalizeWithin(
    matches: FoundMatch[],
    parent: Node,
    removedNodes: Element[],
    clearer: ElementClearer,
): number {
    let childCount = 0;

    for (const { matchType, element } of matches) {
        if (matchType === MatchResult.MATCH) {
            removedNodes.push(element);
            childCount += countChildNodesRespectiveToParent(parent, element);
            element.replaceWith(...element.childNodes);
        } /* matchType === MatchResult.KEEP */ else {
            if (clearer(element)) {
                removedNodes.push(element);
                childCount += countChildNodesRespectiveToParent(parent, element);
                element.replaceWith(...element.childNodes);
            } else {
                childCount += 1;
            }
        }
    }

    const shift = childCount - matches.length;
    return shift;
}

export function normalizeInsertionRanges(
    insertionRanges: ChildNodeRange[],
    matcher: ElementMatcher,
    clearer: ElementClearer,
): NormalizationResult {
    const removedNodes: Element[] = [];
    const normalizedRanges: ChildNodeRange[] = [];

    for (const [index, range] of insertionRanges.entries()) {
        const normalizedRange = { ...range };
        const parent = normalizedRange.parent;

        /**
         * This deals with the unnormalized state that would exist
         * after surrounding and finds conflicting elements, for example cases like:
         * `<b>single<b>double</b>single</b>` or `<i><b>before</b></i><b>after</b>`
         */

        if (index === 0) {
            const matches = findBefore(normalizedRange, matcher);
            const count = normalizeAdjacent(
                matches,
                parent,
                removedNodes,
                matcher,
                clearer,
            );
            normalizedRange.startIndex -= matches.length;
            normalizedRange.endIndex += count - matches.length;
        }

        const matches = findWithin(normalizedRange, matcher);
        const withinShift = normalizeWithin(matches, parent, removedNodes, clearer);
        normalizedRange.endIndex += withinShift;

        if (index === insertionRanges.length - 1) {
            const matches = findAfter(normalizedRange, matcher);
            const count = normalizeAdjacent(
                matches,
                parent,
                removedNodes,
                matcher,
                clearer,
            );
            normalizedRange.endIndex += count;
        }

        normalizedRanges.push(normalizedRange);
    }

    return {
        normalizedRanges,
        removedNodes,
    };
}
