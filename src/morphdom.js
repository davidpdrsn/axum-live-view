/*
 * ATTENTION: The "eval" devtool has been used (maybe by default in mode: "development").
 * This devtool is neither made for production nor for readable output files.
 * It uses "eval()" calls to create a separate source file in the browser devtools.
 * If you are trying to read the output file, select a different devtool (https://webpack.js.org/configuration/devtool/)
 * or disable the default devtool with "devtool: false".
 * If you are looking for production-ready output files, see mode: "production" (https://webpack.js.org/configuration/mode/).
 */
/******/ (() => { // webpackBootstrap
/******/ 	"use strict";
/******/ 	var __webpack_modules__ = ({

/***/ "./liveview-dev.js":
/*!*************************!*\
  !*** ./liveview-dev.js ***!
  \*************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var morphdom__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! morphdom */ \"./node_modules/morphdom/dist/morphdom-esm.js\");\n\nwindow.morphdom = morphdom__WEBPACK_IMPORTED_MODULE_0__[\"default\"];\n\n\n//# sourceURL=webpack://liveview-rust/./liveview-dev.js?");

/***/ }),

/***/ "./node_modules/morphdom/dist/morphdom-esm.js":
/*!****************************************************!*\
  !*** ./node_modules/morphdom/dist/morphdom-esm.js ***!
  \****************************************************/
/***/ ((__unused_webpack_module, __webpack_exports__, __webpack_require__) => {

eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export */ __webpack_require__.d(__webpack_exports__, {\n/* harmony export */   \"default\": () => (__WEBPACK_DEFAULT_EXPORT__)\n/* harmony export */ });\nvar DOCUMENT_FRAGMENT_NODE = 11;\n\nfunction morphAttrs(fromNode, toNode) {\n    var toNodeAttrs = toNode.attributes;\n    var attr;\n    var attrName;\n    var attrNamespaceURI;\n    var attrValue;\n    var fromValue;\n\n    // document-fragments dont have attributes so lets not do anything\n    if (toNode.nodeType === DOCUMENT_FRAGMENT_NODE || fromNode.nodeType === DOCUMENT_FRAGMENT_NODE) {\n      return;\n    }\n\n    // update attributes on original DOM element\n    for (var i = toNodeAttrs.length - 1; i >= 0; i--) {\n        attr = toNodeAttrs[i];\n        attrName = attr.name;\n        attrNamespaceURI = attr.namespaceURI;\n        attrValue = attr.value;\n\n        if (attrNamespaceURI) {\n            attrName = attr.localName || attrName;\n            fromValue = fromNode.getAttributeNS(attrNamespaceURI, attrName);\n\n            if (fromValue !== attrValue) {\n                if (attr.prefix === 'xmlns'){\n                    attrName = attr.name; // It's not allowed to set an attribute with the XMLNS namespace without specifying the `xmlns` prefix\n                }\n                fromNode.setAttributeNS(attrNamespaceURI, attrName, attrValue);\n            }\n        } else {\n            fromValue = fromNode.getAttribute(attrName);\n\n            if (fromValue !== attrValue) {\n                fromNode.setAttribute(attrName, attrValue);\n            }\n        }\n    }\n\n    // Remove any extra attributes found on the original DOM element that\n    // weren't found on the target element.\n    var fromNodeAttrs = fromNode.attributes;\n\n    for (var d = fromNodeAttrs.length - 1; d >= 0; d--) {\n        attr = fromNodeAttrs[d];\n        attrName = attr.name;\n        attrNamespaceURI = attr.namespaceURI;\n\n        if (attrNamespaceURI) {\n            attrName = attr.localName || attrName;\n\n            if (!toNode.hasAttributeNS(attrNamespaceURI, attrName)) {\n                fromNode.removeAttributeNS(attrNamespaceURI, attrName);\n            }\n        } else {\n            if (!toNode.hasAttribute(attrName)) {\n                fromNode.removeAttribute(attrName);\n            }\n        }\n    }\n}\n\nvar range; // Create a range object for efficently rendering strings to elements.\nvar NS_XHTML = 'http://www.w3.org/1999/xhtml';\n\nvar doc = typeof document === 'undefined' ? undefined : document;\nvar HAS_TEMPLATE_SUPPORT = !!doc && 'content' in doc.createElement('template');\nvar HAS_RANGE_SUPPORT = !!doc && doc.createRange && 'createContextualFragment' in doc.createRange();\n\nfunction createFragmentFromTemplate(str) {\n    var template = doc.createElement('template');\n    template.innerHTML = str;\n    return template.content.childNodes[0];\n}\n\nfunction createFragmentFromRange(str) {\n    if (!range) {\n        range = doc.createRange();\n        range.selectNode(doc.body);\n    }\n\n    var fragment = range.createContextualFragment(str);\n    return fragment.childNodes[0];\n}\n\nfunction createFragmentFromWrap(str) {\n    var fragment = doc.createElement('body');\n    fragment.innerHTML = str;\n    return fragment.childNodes[0];\n}\n\n/**\n * This is about the same\n * var html = new DOMParser().parseFromString(str, 'text/html');\n * return html.body.firstChild;\n *\n * @method toElement\n * @param {String} str\n */\nfunction toElement(str) {\n    str = str.trim();\n    if (HAS_TEMPLATE_SUPPORT) {\n      // avoid restrictions on content for things like `<tr><th>Hi</th></tr>` which\n      // createContextualFragment doesn't support\n      // <template> support not available in IE\n      return createFragmentFromTemplate(str);\n    } else if (HAS_RANGE_SUPPORT) {\n      return createFragmentFromRange(str);\n    }\n\n    return createFragmentFromWrap(str);\n}\n\n/**\n * Returns true if two node's names are the same.\n *\n * NOTE: We don't bother checking `namespaceURI` because you will never find two HTML elements with the same\n *       nodeName and different namespace URIs.\n *\n * @param {Element} a\n * @param {Element} b The target element\n * @return {boolean}\n */\nfunction compareNodeNames(fromEl, toEl) {\n    var fromNodeName = fromEl.nodeName;\n    var toNodeName = toEl.nodeName;\n    var fromCodeStart, toCodeStart;\n\n    if (fromNodeName === toNodeName) {\n        return true;\n    }\n\n    fromCodeStart = fromNodeName.charCodeAt(0);\n    toCodeStart = toNodeName.charCodeAt(0);\n\n    // If the target element is a virtual DOM node or SVG node then we may\n    // need to normalize the tag name before comparing. Normal HTML elements that are\n    // in the \"http://www.w3.org/1999/xhtml\"\n    // are converted to upper case\n    if (fromCodeStart <= 90 && toCodeStart >= 97) { // from is upper and to is lower\n        return fromNodeName === toNodeName.toUpperCase();\n    } else if (toCodeStart <= 90 && fromCodeStart >= 97) { // to is upper and from is lower\n        return toNodeName === fromNodeName.toUpperCase();\n    } else {\n        return false;\n    }\n}\n\n/**\n * Create an element, optionally with a known namespace URI.\n *\n * @param {string} name the element name, e.g. 'div' or 'svg'\n * @param {string} [namespaceURI] the element's namespace URI, i.e. the value of\n * its `xmlns` attribute or its inferred namespace.\n *\n * @return {Element}\n */\nfunction createElementNS(name, namespaceURI) {\n    return !namespaceURI || namespaceURI === NS_XHTML ?\n        doc.createElement(name) :\n        doc.createElementNS(namespaceURI, name);\n}\n\n/**\n * Copies the children of one DOM element to another DOM element\n */\nfunction moveChildren(fromEl, toEl) {\n    var curChild = fromEl.firstChild;\n    while (curChild) {\n        var nextChild = curChild.nextSibling;\n        toEl.appendChild(curChild);\n        curChild = nextChild;\n    }\n    return toEl;\n}\n\nfunction syncBooleanAttrProp(fromEl, toEl, name) {\n    if (fromEl[name] !== toEl[name]) {\n        fromEl[name] = toEl[name];\n        if (fromEl[name]) {\n            fromEl.setAttribute(name, '');\n        } else {\n            fromEl.removeAttribute(name);\n        }\n    }\n}\n\nvar specialElHandlers = {\n    OPTION: function(fromEl, toEl) {\n        var parentNode = fromEl.parentNode;\n        if (parentNode) {\n            var parentName = parentNode.nodeName.toUpperCase();\n            if (parentName === 'OPTGROUP') {\n                parentNode = parentNode.parentNode;\n                parentName = parentNode && parentNode.nodeName.toUpperCase();\n            }\n            if (parentName === 'SELECT' && !parentNode.hasAttribute('multiple')) {\n                if (fromEl.hasAttribute('selected') && !toEl.selected) {\n                    // Workaround for MS Edge bug where the 'selected' attribute can only be\n                    // removed if set to a non-empty value:\n                    // https://developer.microsoft.com/en-us/microsoft-edge/platform/issues/12087679/\n                    fromEl.setAttribute('selected', 'selected');\n                    fromEl.removeAttribute('selected');\n                }\n                // We have to reset select element's selectedIndex to -1, otherwise setting\n                // fromEl.selected using the syncBooleanAttrProp below has no effect.\n                // The correct selectedIndex will be set in the SELECT special handler below.\n                parentNode.selectedIndex = -1;\n            }\n        }\n        syncBooleanAttrProp(fromEl, toEl, 'selected');\n    },\n    /**\n     * The \"value\" attribute is special for the <input> element since it sets\n     * the initial value. Changing the \"value\" attribute without changing the\n     * \"value\" property will have no effect since it is only used to the set the\n     * initial value.  Similar for the \"checked\" attribute, and \"disabled\".\n     */\n    INPUT: function(fromEl, toEl) {\n        syncBooleanAttrProp(fromEl, toEl, 'checked');\n        syncBooleanAttrProp(fromEl, toEl, 'disabled');\n\n        if (fromEl.value !== toEl.value) {\n            fromEl.value = toEl.value;\n        }\n\n        if (!toEl.hasAttribute('value')) {\n            fromEl.removeAttribute('value');\n        }\n    },\n\n    TEXTAREA: function(fromEl, toEl) {\n        var newValue = toEl.value;\n        if (fromEl.value !== newValue) {\n            fromEl.value = newValue;\n        }\n\n        var firstChild = fromEl.firstChild;\n        if (firstChild) {\n            // Needed for IE. Apparently IE sets the placeholder as the\n            // node value and vise versa. This ignores an empty update.\n            var oldValue = firstChild.nodeValue;\n\n            if (oldValue == newValue || (!newValue && oldValue == fromEl.placeholder)) {\n                return;\n            }\n\n            firstChild.nodeValue = newValue;\n        }\n    },\n    SELECT: function(fromEl, toEl) {\n        if (!toEl.hasAttribute('multiple')) {\n            var selectedIndex = -1;\n            var i = 0;\n            // We have to loop through children of fromEl, not toEl since nodes can be moved\n            // from toEl to fromEl directly when morphing.\n            // At the time this special handler is invoked, all children have already been morphed\n            // and appended to / removed from fromEl, so using fromEl here is safe and correct.\n            var curChild = fromEl.firstChild;\n            var optgroup;\n            var nodeName;\n            while(curChild) {\n                nodeName = curChild.nodeName && curChild.nodeName.toUpperCase();\n                if (nodeName === 'OPTGROUP') {\n                    optgroup = curChild;\n                    curChild = optgroup.firstChild;\n                } else {\n                    if (nodeName === 'OPTION') {\n                        if (curChild.hasAttribute('selected')) {\n                            selectedIndex = i;\n                            break;\n                        }\n                        i++;\n                    }\n                    curChild = curChild.nextSibling;\n                    if (!curChild && optgroup) {\n                        curChild = optgroup.nextSibling;\n                        optgroup = null;\n                    }\n                }\n            }\n\n            fromEl.selectedIndex = selectedIndex;\n        }\n    }\n};\n\nvar ELEMENT_NODE = 1;\nvar DOCUMENT_FRAGMENT_NODE$1 = 11;\nvar TEXT_NODE = 3;\nvar COMMENT_NODE = 8;\n\nfunction noop() {}\n\nfunction defaultGetNodeKey(node) {\n  if (node) {\n      return (node.getAttribute && node.getAttribute('id')) || node.id;\n  }\n}\n\nfunction morphdomFactory(morphAttrs) {\n\n    return function morphdom(fromNode, toNode, options) {\n        if (!options) {\n            options = {};\n        }\n\n        if (typeof toNode === 'string') {\n            if (fromNode.nodeName === '#document' || fromNode.nodeName === 'HTML' || fromNode.nodeName === 'BODY') {\n                var toNodeHtml = toNode;\n                toNode = doc.createElement('html');\n                toNode.innerHTML = toNodeHtml;\n            } else {\n                toNode = toElement(toNode);\n            }\n        }\n\n        var getNodeKey = options.getNodeKey || defaultGetNodeKey;\n        var onBeforeNodeAdded = options.onBeforeNodeAdded || noop;\n        var onNodeAdded = options.onNodeAdded || noop;\n        var onBeforeElUpdated = options.onBeforeElUpdated || noop;\n        var onElUpdated = options.onElUpdated || noop;\n        var onBeforeNodeDiscarded = options.onBeforeNodeDiscarded || noop;\n        var onNodeDiscarded = options.onNodeDiscarded || noop;\n        var onBeforeElChildrenUpdated = options.onBeforeElChildrenUpdated || noop;\n        var childrenOnly = options.childrenOnly === true;\n\n        // This object is used as a lookup to quickly find all keyed elements in the original DOM tree.\n        var fromNodesLookup = Object.create(null);\n        var keyedRemovalList = [];\n\n        function addKeyedRemoval(key) {\n            keyedRemovalList.push(key);\n        }\n\n        function walkDiscardedChildNodes(node, skipKeyedNodes) {\n            if (node.nodeType === ELEMENT_NODE) {\n                var curChild = node.firstChild;\n                while (curChild) {\n\n                    var key = undefined;\n\n                    if (skipKeyedNodes && (key = getNodeKey(curChild))) {\n                        // If we are skipping keyed nodes then we add the key\n                        // to a list so that it can be handled at the very end.\n                        addKeyedRemoval(key);\n                    } else {\n                        // Only report the node as discarded if it is not keyed. We do this because\n                        // at the end we loop through all keyed elements that were unmatched\n                        // and then discard them in one final pass.\n                        onNodeDiscarded(curChild);\n                        if (curChild.firstChild) {\n                            walkDiscardedChildNodes(curChild, skipKeyedNodes);\n                        }\n                    }\n\n                    curChild = curChild.nextSibling;\n                }\n            }\n        }\n\n        /**\n         * Removes a DOM node out of the original DOM\n         *\n         * @param  {Node} node The node to remove\n         * @param  {Node} parentNode The nodes parent\n         * @param  {Boolean} skipKeyedNodes If true then elements with keys will be skipped and not discarded.\n         * @return {undefined}\n         */\n        function removeNode(node, parentNode, skipKeyedNodes) {\n            if (onBeforeNodeDiscarded(node) === false) {\n                return;\n            }\n\n            if (parentNode) {\n                parentNode.removeChild(node);\n            }\n\n            onNodeDiscarded(node);\n            walkDiscardedChildNodes(node, skipKeyedNodes);\n        }\n\n        // // TreeWalker implementation is no faster, but keeping this around in case this changes in the future\n        // function indexTree(root) {\n        //     var treeWalker = document.createTreeWalker(\n        //         root,\n        //         NodeFilter.SHOW_ELEMENT);\n        //\n        //     var el;\n        //     while((el = treeWalker.nextNode())) {\n        //         var key = getNodeKey(el);\n        //         if (key) {\n        //             fromNodesLookup[key] = el;\n        //         }\n        //     }\n        // }\n\n        // // NodeIterator implementation is no faster, but keeping this around in case this changes in the future\n        //\n        // function indexTree(node) {\n        //     var nodeIterator = document.createNodeIterator(node, NodeFilter.SHOW_ELEMENT);\n        //     var el;\n        //     while((el = nodeIterator.nextNode())) {\n        //         var key = getNodeKey(el);\n        //         if (key) {\n        //             fromNodesLookup[key] = el;\n        //         }\n        //     }\n        // }\n\n        function indexTree(node) {\n            if (node.nodeType === ELEMENT_NODE || node.nodeType === DOCUMENT_FRAGMENT_NODE$1) {\n                var curChild = node.firstChild;\n                while (curChild) {\n                    var key = getNodeKey(curChild);\n                    if (key) {\n                        fromNodesLookup[key] = curChild;\n                    }\n\n                    // Walk recursively\n                    indexTree(curChild);\n\n                    curChild = curChild.nextSibling;\n                }\n            }\n        }\n\n        indexTree(fromNode);\n\n        function handleNodeAdded(el) {\n            onNodeAdded(el);\n\n            var curChild = el.firstChild;\n            while (curChild) {\n                var nextSibling = curChild.nextSibling;\n\n                var key = getNodeKey(curChild);\n                if (key) {\n                    var unmatchedFromEl = fromNodesLookup[key];\n                    // if we find a duplicate #id node in cache, replace `el` with cache value\n                    // and morph it to the child node.\n                    if (unmatchedFromEl && compareNodeNames(curChild, unmatchedFromEl)) {\n                        curChild.parentNode.replaceChild(unmatchedFromEl, curChild);\n                        morphEl(unmatchedFromEl, curChild);\n                    } else {\n                      handleNodeAdded(curChild);\n                    }\n                } else {\n                  // recursively call for curChild and it's children to see if we find something in\n                  // fromNodesLookup\n                  handleNodeAdded(curChild);\n                }\n\n                curChild = nextSibling;\n            }\n        }\n\n        function cleanupFromEl(fromEl, curFromNodeChild, curFromNodeKey) {\n            // We have processed all of the \"to nodes\". If curFromNodeChild is\n            // non-null then we still have some from nodes left over that need\n            // to be removed\n            while (curFromNodeChild) {\n                var fromNextSibling = curFromNodeChild.nextSibling;\n                if ((curFromNodeKey = getNodeKey(curFromNodeChild))) {\n                    // Since the node is keyed it might be matched up later so we defer\n                    // the actual removal to later\n                    addKeyedRemoval(curFromNodeKey);\n                } else {\n                    // NOTE: we skip nested keyed nodes from being removed since there is\n                    //       still a chance they will be matched up later\n                    removeNode(curFromNodeChild, fromEl, true /* skip keyed nodes */);\n                }\n                curFromNodeChild = fromNextSibling;\n            }\n        }\n\n        function morphEl(fromEl, toEl, childrenOnly) {\n            var toElKey = getNodeKey(toEl);\n\n            if (toElKey) {\n                // If an element with an ID is being morphed then it will be in the final\n                // DOM so clear it out of the saved elements collection\n                delete fromNodesLookup[toElKey];\n            }\n\n            if (!childrenOnly) {\n                // optional\n                if (onBeforeElUpdated(fromEl, toEl) === false) {\n                    return;\n                }\n\n                // update attributes on original DOM element first\n                morphAttrs(fromEl, toEl);\n                // optional\n                onElUpdated(fromEl);\n\n                if (onBeforeElChildrenUpdated(fromEl, toEl) === false) {\n                    return;\n                }\n            }\n\n            if (fromEl.nodeName !== 'TEXTAREA') {\n              morphChildren(fromEl, toEl);\n            } else {\n              specialElHandlers.TEXTAREA(fromEl, toEl);\n            }\n        }\n\n        function morphChildren(fromEl, toEl) {\n            var curToNodeChild = toEl.firstChild;\n            var curFromNodeChild = fromEl.firstChild;\n            var curToNodeKey;\n            var curFromNodeKey;\n\n            var fromNextSibling;\n            var toNextSibling;\n            var matchingFromEl;\n\n            // walk the children\n            outer: while (curToNodeChild) {\n                toNextSibling = curToNodeChild.nextSibling;\n                curToNodeKey = getNodeKey(curToNodeChild);\n\n                // walk the fromNode children all the way through\n                while (curFromNodeChild) {\n                    fromNextSibling = curFromNodeChild.nextSibling;\n\n                    if (curToNodeChild.isSameNode && curToNodeChild.isSameNode(curFromNodeChild)) {\n                        curToNodeChild = toNextSibling;\n                        curFromNodeChild = fromNextSibling;\n                        continue outer;\n                    }\n\n                    curFromNodeKey = getNodeKey(curFromNodeChild);\n\n                    var curFromNodeType = curFromNodeChild.nodeType;\n\n                    // this means if the curFromNodeChild doesnt have a match with the curToNodeChild\n                    var isCompatible = undefined;\n\n                    if (curFromNodeType === curToNodeChild.nodeType) {\n                        if (curFromNodeType === ELEMENT_NODE) {\n                            // Both nodes being compared are Element nodes\n\n                            if (curToNodeKey) {\n                                // The target node has a key so we want to match it up with the correct element\n                                // in the original DOM tree\n                                if (curToNodeKey !== curFromNodeKey) {\n                                    // The current element in the original DOM tree does not have a matching key so\n                                    // let's check our lookup to see if there is a matching element in the original\n                                    // DOM tree\n                                    if ((matchingFromEl = fromNodesLookup[curToNodeKey])) {\n                                        if (fromNextSibling === matchingFromEl) {\n                                            // Special case for single element removals. To avoid removing the original\n                                            // DOM node out of the tree (since that can break CSS transitions, etc.),\n                                            // we will instead discard the current node and wait until the next\n                                            // iteration to properly match up the keyed target element with its matching\n                                            // element in the original tree\n                                            isCompatible = false;\n                                        } else {\n                                            // We found a matching keyed element somewhere in the original DOM tree.\n                                            // Let's move the original DOM node into the current position and morph\n                                            // it.\n\n                                            // NOTE: We use insertBefore instead of replaceChild because we want to go through\n                                            // the `removeNode()` function for the node that is being discarded so that\n                                            // all lifecycle hooks are correctly invoked\n                                            fromEl.insertBefore(matchingFromEl, curFromNodeChild);\n\n                                            // fromNextSibling = curFromNodeChild.nextSibling;\n\n                                            if (curFromNodeKey) {\n                                                // Since the node is keyed it might be matched up later so we defer\n                                                // the actual removal to later\n                                                addKeyedRemoval(curFromNodeKey);\n                                            } else {\n                                                // NOTE: we skip nested keyed nodes from being removed since there is\n                                                //       still a chance they will be matched up later\n                                                removeNode(curFromNodeChild, fromEl, true /* skip keyed nodes */);\n                                            }\n\n                                            curFromNodeChild = matchingFromEl;\n                                        }\n                                    } else {\n                                        // The nodes are not compatible since the \"to\" node has a key and there\n                                        // is no matching keyed node in the source tree\n                                        isCompatible = false;\n                                    }\n                                }\n                            } else if (curFromNodeKey) {\n                                // The original has a key\n                                isCompatible = false;\n                            }\n\n                            isCompatible = isCompatible !== false && compareNodeNames(curFromNodeChild, curToNodeChild);\n                            if (isCompatible) {\n                                // We found compatible DOM elements so transform\n                                // the current \"from\" node to match the current\n                                // target DOM node.\n                                // MORPH\n                                morphEl(curFromNodeChild, curToNodeChild);\n                            }\n\n                        } else if (curFromNodeType === TEXT_NODE || curFromNodeType == COMMENT_NODE) {\n                            // Both nodes being compared are Text or Comment nodes\n                            isCompatible = true;\n                            // Simply update nodeValue on the original node to\n                            // change the text value\n                            if (curFromNodeChild.nodeValue !== curToNodeChild.nodeValue) {\n                                curFromNodeChild.nodeValue = curToNodeChild.nodeValue;\n                            }\n\n                        }\n                    }\n\n                    if (isCompatible) {\n                        // Advance both the \"to\" child and the \"from\" child since we found a match\n                        // Nothing else to do as we already recursively called morphChildren above\n                        curToNodeChild = toNextSibling;\n                        curFromNodeChild = fromNextSibling;\n                        continue outer;\n                    }\n\n                    // No compatible match so remove the old node from the DOM and continue trying to find a\n                    // match in the original DOM. However, we only do this if the from node is not keyed\n                    // since it is possible that a keyed node might match up with a node somewhere else in the\n                    // target tree and we don't want to discard it just yet since it still might find a\n                    // home in the final DOM tree. After everything is done we will remove any keyed nodes\n                    // that didn't find a home\n                    if (curFromNodeKey) {\n                        // Since the node is keyed it might be matched up later so we defer\n                        // the actual removal to later\n                        addKeyedRemoval(curFromNodeKey);\n                    } else {\n                        // NOTE: we skip nested keyed nodes from being removed since there is\n                        //       still a chance they will be matched up later\n                        removeNode(curFromNodeChild, fromEl, true /* skip keyed nodes */);\n                    }\n\n                    curFromNodeChild = fromNextSibling;\n                } // END: while(curFromNodeChild) {}\n\n                // If we got this far then we did not find a candidate match for\n                // our \"to node\" and we exhausted all of the children \"from\"\n                // nodes. Therefore, we will just append the current \"to\" node\n                // to the end\n                if (curToNodeKey && (matchingFromEl = fromNodesLookup[curToNodeKey]) && compareNodeNames(matchingFromEl, curToNodeChild)) {\n                    fromEl.appendChild(matchingFromEl);\n                    // MORPH\n                    morphEl(matchingFromEl, curToNodeChild);\n                } else {\n                    var onBeforeNodeAddedResult = onBeforeNodeAdded(curToNodeChild);\n                    if (onBeforeNodeAddedResult !== false) {\n                        if (onBeforeNodeAddedResult) {\n                            curToNodeChild = onBeforeNodeAddedResult;\n                        }\n\n                        if (curToNodeChild.actualize) {\n                            curToNodeChild = curToNodeChild.actualize(fromEl.ownerDocument || doc);\n                        }\n                        fromEl.appendChild(curToNodeChild);\n                        handleNodeAdded(curToNodeChild);\n                    }\n                }\n\n                curToNodeChild = toNextSibling;\n                curFromNodeChild = fromNextSibling;\n            }\n\n            cleanupFromEl(fromEl, curFromNodeChild, curFromNodeKey);\n\n            var specialElHandler = specialElHandlers[fromEl.nodeName];\n            if (specialElHandler) {\n                specialElHandler(fromEl, toEl);\n            }\n        } // END: morphChildren(...)\n\n        var morphedNode = fromNode;\n        var morphedNodeType = morphedNode.nodeType;\n        var toNodeType = toNode.nodeType;\n\n        if (!childrenOnly) {\n            // Handle the case where we are given two DOM nodes that are not\n            // compatible (e.g. <div> --> <span> or <div> --> TEXT)\n            if (morphedNodeType === ELEMENT_NODE) {\n                if (toNodeType === ELEMENT_NODE) {\n                    if (!compareNodeNames(fromNode, toNode)) {\n                        onNodeDiscarded(fromNode);\n                        morphedNode = moveChildren(fromNode, createElementNS(toNode.nodeName, toNode.namespaceURI));\n                    }\n                } else {\n                    // Going from an element node to a text node\n                    morphedNode = toNode;\n                }\n            } else if (morphedNodeType === TEXT_NODE || morphedNodeType === COMMENT_NODE) { // Text or comment node\n                if (toNodeType === morphedNodeType) {\n                    if (morphedNode.nodeValue !== toNode.nodeValue) {\n                        morphedNode.nodeValue = toNode.nodeValue;\n                    }\n\n                    return morphedNode;\n                } else {\n                    // Text node to something else\n                    morphedNode = toNode;\n                }\n            }\n        }\n\n        if (morphedNode === toNode) {\n            // The \"to node\" was not compatible with the \"from node\" so we had to\n            // toss out the \"from node\" and use the \"to node\"\n            onNodeDiscarded(fromNode);\n        } else {\n            if (toNode.isSameNode && toNode.isSameNode(morphedNode)) {\n                return;\n            }\n\n            morphEl(morphedNode, toNode, childrenOnly);\n\n            // We now need to loop over any keyed nodes that might need to be\n            // removed. We only do the removal if we know that the keyed node\n            // never found a match. When a keyed node is matched up we remove\n            // it out of fromNodesLookup and we use fromNodesLookup to determine\n            // if a keyed node has been matched up or not\n            if (keyedRemovalList) {\n                for (var i=0, len=keyedRemovalList.length; i<len; i++) {\n                    var elToRemove = fromNodesLookup[keyedRemovalList[i]];\n                    if (elToRemove) {\n                        removeNode(elToRemove, elToRemove.parentNode, false);\n                    }\n                }\n            }\n        }\n\n        if (!childrenOnly && morphedNode !== fromNode && fromNode.parentNode) {\n            if (morphedNode.actualize) {\n                morphedNode = morphedNode.actualize(fromNode.ownerDocument || doc);\n            }\n            // If we had to swap out the from node with a new node because the old\n            // node was not compatible with the target node then we need to\n            // replace the old DOM node in the original DOM tree. This is only\n            // possible if the original DOM node was part of a DOM tree which\n            // we know is the case if it has a parent node.\n            fromNode.parentNode.replaceChild(morphedNode, fromNode);\n        }\n\n        return morphedNode;\n    };\n}\n\nvar morphdom = morphdomFactory(morphAttrs);\n\n/* harmony default export */ const __WEBPACK_DEFAULT_EXPORT__ = (morphdom);\n\n\n//# sourceURL=webpack://liveview-rust/./node_modules/morphdom/dist/morphdom-esm.js?");

/***/ })

/******/ 	});
/************************************************************************/
/******/ 	// The module cache
/******/ 	var __webpack_module_cache__ = {};
/******/ 	
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/ 		// Check if module is in cache
/******/ 		var cachedModule = __webpack_module_cache__[moduleId];
/******/ 		if (cachedModule !== undefined) {
/******/ 			return cachedModule.exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = __webpack_module_cache__[moduleId] = {
/******/ 			// no module.id needed
/******/ 			// no module.loaded needed
/******/ 			exports: {}
/******/ 		};
/******/ 	
/******/ 		// Execute the module function
/******/ 		__webpack_modules__[moduleId](module, module.exports, __webpack_require__);
/******/ 	
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/ 	
/************************************************************************/
/******/ 	/* webpack/runtime/define property getters */
/******/ 	(() => {
/******/ 		// define getter functions for harmony exports
/******/ 		__webpack_require__.d = (exports, definition) => {
/******/ 			for(var key in definition) {
/******/ 				if(__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) {
/******/ 					Object.defineProperty(exports, key, { enumerable: true, get: definition[key] });
/******/ 				}
/******/ 			}
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/hasOwnProperty shorthand */
/******/ 	(() => {
/******/ 		__webpack_require__.o = (obj, prop) => (Object.prototype.hasOwnProperty.call(obj, prop))
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/make namespace object */
/******/ 	(() => {
/******/ 		// define __esModule on exports
/******/ 		__webpack_require__.r = (exports) => {
/******/ 			if(typeof Symbol !== 'undefined' && Symbol.toStringTag) {
/******/ 				Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
/******/ 			}
/******/ 			Object.defineProperty(exports, '__esModule', { value: true });
/******/ 		};
/******/ 	})();
/******/ 	
/************************************************************************/
/******/ 	
/******/ 	// startup
/******/ 	// Load entry module and return exports
/******/ 	// This entry module can't be inlined because the eval devtool is used.
/******/ 	var __webpack_exports__ = __webpack_require__("./liveview-dev.js");
/******/ 	
/******/ })()
;