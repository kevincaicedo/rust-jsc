use rust_jsc::{
    JSArrayBuffer, JSContext, JSResult, JSTypedArray, JSTypedArrayType, JSValue,
};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();

    let copied =
        JSTypedArray::with_bytes(&ctx, &[1u16, 2, 3, 4], JSTypedArrayType::Uint16Array)?;
    assert_eq!(copied.len()?, 4);
    assert_eq!(copied.byte_len()?, 8);
    assert_eq!(copied.as_vec::<u16>()?, vec![1, 2, 3, 4]);

    let owned = JSTypedArray::with_owned_bytes(
        &ctx,
        vec![10u8, 20, 30, 40],
        JSTypedArrayType::Uint8Array,
    )?;
    let owned_value: JSValue = owned.clone().into();
    ctx.global_object()
        .set_property("input", &owned_value, Default::default())?;

    let sum = ctx
        .evaluate_script(
            r#"
            let total = 0;
            for (const byte of input) total += byte;
            total;
            "#,
            None,
        )?
        .as_number()?;
    assert_eq!(sum, 100.0);
    assert_eq!(owned.as_vec::<u8>()?, vec![10, 20, 30, 40]);

    let buffer = JSArrayBuffer::from_vec(&ctx, vec![9, 8, 7, 6])?;
    assert_eq!(buffer.len()?, 4);
    assert_eq!(buffer.as_vec()?, vec![9, 8, 7, 6]);

    println!("typed arrays: copied={} owned_sum={sum}", copied.len()?);
    Ok(())
}
