//! Embedding module - Feature hashing text vectorization
//!
//! Zero-dependency text-to-vector conversion using feature hashing.
//! Produces deterministic 768-dimensional unit vectors for cosine similarity.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Vector dimension for embeddings (matches common transformer output sizes)
pub const DIM: usize = 768;

/// Convert text to a 768-dimensional feature-hashed vector.
///
/// Process:
/// 1. Lowercase the text
/// 2. Extract unigrams and bigrams
/// 3. Hash each token to a dimension index using DefaultHasher
/// 4. Apply term frequency weighting
/// 5. L2 normalize to unit vector
///
/// # Arguments
/// * `text` - Input text to vectorize
///
/// # Returns
/// 768-dimensional f32 vector, L2-normalized
///
/// # Determinism
/// Same input always produces same output (DefaultHasher is deterministic)
pub fn text_to_vector(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0f32; DIM];

    // Lowercase and tokenize
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();

    if chars.is_empty() {
        return vector;
    }

    // Extract unigrams and bigrams
    let mut tokens: Vec<String> = Vec::new();

    // Unigrams
    for ch in &chars {
        if ch.is_alphanumeric() {
            tokens.push(ch.to_string());
        }
    }

    // Bigrams
    for pair in chars.windows(2) {
        if pair[0].is_alphanumeric() && pair[1].is_alphanumeric() {
            let bigram: String = [pair[0], pair[1]].iter().collect();
            tokens.push(bigram);
        }
    }

    // Hash tokens and accumulate term frequencies
    for token in tokens {
        let mut hasher = DefaultHasher::new();
        token.hash(&mut hasher);
        let hash = hasher.finish();
        let dim = (hash % DIM as u64) as usize;

        // Term frequency weighting
        vector[dim] += 1.0;
    }

    // L2 normalize
    let norm: f32 = vector.iter().map(|&x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in vector.iter_mut() {
            *v /= norm;
        }
    }

    vector
}

/// Compute cosine similarity between two L2-normalized vectors.
///
/// Since vectors are L2-normalized, cosine similarity equals dot product.
///
/// # Arguments
/// * `a` - First vector (must be L2-normalized)
/// * `b` - Second vector (must be L2-normalized)
///
/// # Returns
/// Cosine similarity score (0.0 = orthogonal, 1.0 = identical)
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    // Dot product of L2-normalized vectors equals cosine similarity
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Serialize a vector to bytes (little-endian f32).
///
/// # Arguments
/// * `vec` - Vector to serialize
///
/// # Returns
/// Byte representation (4 bytes per f32)
pub fn vector_to_blob(vec: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(vec.len() * 4);
    for &val in vec {
        blob.extend_from_slice(&val.to_le_bytes());
    }
    blob
}

/// Deserialize a vector from bytes (little-endian f32).
///
/// # Arguments
/// * `blob` - Byte array to deserialize (must be multiple of 4)
///
/// # Returns
/// Deserialized f32 vector
pub fn blob_to_vector(blob: &[u8]) -> Vec<f32> {
    if !blob.len().is_multiple_of(4) {
        return Vec::new();
    }

    let mut vec = Vec::with_capacity(blob.len() / 4);
    for chunk in blob.chunks_exact(4) {
        let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
        let val = f32::from_le_bytes(bytes);
        vec.push(val);
    }
    vec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determinism() {
        let text = "Hello World";
        let vec1 = text_to_vector(text);
        let vec2 = text_to_vector(text);

        assert_eq!(vec1.len(), DIM);
        assert_eq!(vec2.len(), DIM);
        assert_eq!(vec1, vec2, "Same text must produce same vector");
    }

    #[test]
    fn test_identical_vectors_cosine_one() {
        let text = "test text";
        let vec = text_to_vector(text);
        let similarity = cosine_similarity(&vec, &vec);

        // Use slightly larger tolerance for floating point precision
        assert!(
            (similarity - 1.0).abs() < 1e-5,
            "Cosine similarity of identical vectors must be 1.0, got {}",
            similarity
        );
    }

    #[test]
    fn test_orthogonal_vectors_cosine_zero() {
        // Create orthogonal vectors by using different text
        let vec1 = text_to_vector("aaaaaaaaaa");
        let vec2 = text_to_vector("bbbbbbbbbb");
        let similarity = cosine_similarity(&vec1, &vec2);

        // Should be near 0 (feature hashing may cause some collisions)
        assert!(
            similarity.abs() < 0.3,
            "Cosine similarity of different text should be near 0, got {}",
            similarity
        );
    }

    #[test]
    fn test_blob_round_trip() {
        let text = "round trip test";
        let original = text_to_vector(text);
        let blob = vector_to_blob(&original);
        let restored = blob_to_vector(&blob);

        assert_eq!(original.len(), restored.len());
        for (i, (orig, rest)) in original.iter().zip(restored.iter()).enumerate() {
            assert!(
                (orig - rest).abs() < f32::EPSILON,
                "Round trip failed at index {}: {} vs {}",
                i,
                orig,
                rest
            );
        }
    }

    #[test]
    fn test_l2_norm_unit_vector() {
        let text = "normalization test with some text here";
        let vec = text_to_vector(text);

        let norm_sq: f32 = vec.iter().map(|&x| x * x).sum::<f32>();
        let norm = norm_sq.sqrt();

        assert!(
            (norm - 1.0).abs() < 1e-5,
            "L2 norm must be 1.0 for unit vector, got {} (norm_sq = {})",
            norm,
            norm_sq
        );
    }

    #[test]
    fn test_empty_text() {
        let vec = text_to_vector("");

        assert_eq!(vec.len(), DIM);
        // All zeros since no tokens to hash
        for &val in &vec {
            assert_eq!(val, 0.0);
        }
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let vec1 = vec![0.5f32; 100];
        let vec2 = vec![0.5f32; 200];

        let similarity = cosine_similarity(&vec1, &vec2);
        assert_eq!(
            similarity, 0.0,
            "Different length vectors should return 0.0"
        );
    }

    #[test]
    fn test_blob_to_vector_invalid_length() {
        let invalid_blob = vec![1u8, 2, 3]; // Not multiple of 4
        let vec = blob_to_vector(&invalid_blob);

        assert_eq!(
            vec.len(),
            0,
            "Invalid blob length should return empty vector"
        );
    }

    #[test]
    fn test_text_case_insensitive() {
        let vec1 = text_to_vector("Hello World");
        let vec2 = text_to_vector("HELLO WORLD");
        let vec3 = text_to_vector("hello world");

        // All should be identical due to lowercase normalization
        assert_eq!(vec1, vec2);
        assert_eq!(vec2, vec3);
    }

    #[test]
    fn test_bigram_extraction() {
        // Text with adjacent alphanumeric chars should produce bigrams
        let text = "ab cd";
        let vec = text_to_vector(text);

        // Should not be all zeros
        let has_nonzero = vec.iter().any(|&x| x > 0.0);
        assert!(
            has_nonzero,
            "Vector should have non-zero values from bigrams"
        );
    }

    #[test]
    fn test_vector_to_blob_size() {
        let vec = vec![1.0f32; DIM];
        let blob = vector_to_blob(&vec);

        assert_eq!(blob.len(), DIM * 4, "Blob should be 4 bytes per f32");
    }

    #[test]
    fn test_same_text_different_instances() {
        // Multiple calls should be deterministic
        let text = "deterministic hashing test";
        let vectors: Vec<Vec<f32>> = (0..10).map(|_| text_to_vector(text)).collect();

        // All vectors should be identical
        for vec in &vectors[1..] {
            assert_eq!(&vectors[0], vec);
        }
    }
}
