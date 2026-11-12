package com.dusimagez

/**
 * Minimal JSON parser for `shared/test-vectors.json` — just enough
 * for the vectors schema (objects, arrays, strings, numbers,
 * booleans, null). No third-party dependency: `org.json` is not
 * reliably available in local JVM unit tests, and the vectors file
 * is small. Numbers decode to Long when integral, Double otherwise.
 * Throws [IllegalArgumentException] on malformed input.
 */
object TestJson {

    fun parse(text: String): Any? = Parser(text).parseValue().also { /* trailing ws ok */ }

    @Suppress("UNCHECKED_CAST")
    fun obj(value: Any?): Map<String, Any?> = value as? Map<String, Any?> ?: emptyMap()

    @Suppress("UNCHECKED_CAST")
    fun array(value: Any?): List<Any?> = value as? List<Any?> ?: emptyList()

    fun string(value: Any?): String? = value as? String

    fun long(value: Any?): Long? = when (value) {
        is Long -> value
        is Double -> if (value == value.toLong().toDouble()) value.toLong() else null
        else -> null
    }

    fun double(value: Any?): Double? = when (value) {
        is Double -> value
        is Long -> value.toDouble()
        else -> null
    }

    fun bool(value: Any?): Boolean? = value as? Boolean

    private class Parser(private val text: String) {
        private var pos = 0

        fun parseValue(): Any? {
            skipWs()
            return when (val c = peek()) {
                '{' -> parseObject()
                '[' -> parseArray()
                '"' -> parseString()
                't' -> { expect("true"); true }
                'f' -> { expect("false"); false }
                'n' -> { expect("null"); null }
                else -> parseNumber(c)
            }
        }

        private fun parseObject(): Map<String, Any?> {
            expectChar('{')
            val map = LinkedHashMap<String, Any?>()
            skipWs()
            if (peek() == '}') {
                pos++
                return map
            }
            while (true) {
                skipWs()
                val key = parseString()
                skipWs()
                expectChar(':')
                map[key] = parseValue()
                skipWs()
                when (peek()) {
                    ',' -> pos++
                    '}' -> {
                        pos++
                        return map
                    }
                    else -> error("expected ',' or '}' in object at $pos")
                }
            }
        }

        private fun parseArray(): List<Any?> {
            expectChar('[')
            val list = ArrayList<Any?>()
            skipWs()
            if (peek() == ']') {
                pos++
                return list
            }
            while (true) {
                list.add(parseValue())
                skipWs()
                when (peek()) {
                    ',' -> pos++
                    ']' -> {
                        pos++
                        return list
                    }
                    else -> error("expected ',' or ']' in array at $pos")
                }
            }
        }

        private fun parseString(): String {
            expectChar('"')
            val sb = StringBuilder()
            while (true) {
                if (pos >= text.length) error("unterminated string")
                val c = text[pos++]
                when (c) {
                    '"' -> return sb.toString()
                    '\\' -> {
                        if (pos >= text.length) error("unterminated escape")
                        when (val e = text[pos++]) {
                            '"', '\\', '/' -> sb.append(e)
                            'b' -> sb.append('\b')
                            'f' -> sb.append('\u000C')
                            'n' -> sb.append('\n')
                            'r' -> sb.append('\r')
                            't' -> sb.append('\t')
                            'u' -> {
                                if (pos + 4 > text.length) error("bad \\u escape")
                                sb.append(text.substring(pos, pos + 4).toInt(16).toChar())
                                pos += 4
                            }
                            else -> error("bad escape \\$e")
                        }
                    }
                    else -> sb.append(c)
                }
            }
        }

        private fun parseNumber(first: Char): Any {
            val start = pos
            if (first == '-') pos++
            while (pos < text.length && (text[pos].isDigit() || text[pos] in ".eE+-")) pos++
            val raw = text.substring(start, pos)
            return raw.toLongOrNull() ?: raw.toDoubleOrNull()
                ?: error("bad number '$raw'")
        }

        private fun skipWs() {
            while (pos < text.length && text[pos].isWhitespace()) pos++
        }

        private fun peek(): Char {
            if (pos >= text.length) error("unexpected end of input at $pos")
            return text[pos]
        }

        private fun expectChar(c: Char) {
            skipWs()
            if (pos >= text.length || text[pos] != c) error("expected '$c' at $pos")
            pos++
        }

        private fun expect(word: String) {
            if (!text.startsWith(word, pos)) error("expected '$word' at $pos")
            pos += word.length
        }
    }
}
